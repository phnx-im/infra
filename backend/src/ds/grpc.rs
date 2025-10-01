// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::{
    credentials::{ClientCredential, keys::ClientVerifyingKey},
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{
            keys::LeafVerifyingKeyRef,
            private_keys::SignatureVerificationError,
            signable::{Verifiable, VerifiedStruct},
        },
    },
    identifiers::{self, AttachmentId, Fqdn, QualifiedGroupId},
    messages::client_ds::{
        GroupOperationParams, JoinConnectionGroupParams, QsQueueMessagePayload,
        UserProfileKeyUpdateParams, WelcomeInfoParams,
    },
    time::TimeStamp,
};
use airprotos::{
    convert::{RefInto, TryFromRef as _, TryRefInto},
    delivery_service::v1::{self, delivery_service_server::DeliveryService, *},
    validation::{InvalidTlsExt, MissingFieldExt},
};
use chrono::TimeDelta;
use mimi_room_policy::VerifiedRoomState;
use mls_assist::{
    group::Group,
    messages::{AssistedMessageIn, SerializedMlsMessage},
    openmls::prelude::{LeafNodeIndex, MlsMessageBodyIn, MlsMessageIn, RatchetTreeIn, Sender},
};
use thiserror::Error;
use tls_codec::DeserializeBytes;
use tokio::task::{JoinError, JoinSet};
use tonic::{Request, Response, Status, async_trait};
use tracing::{error, warn};

use crate::{
    ds::process::Provider,
    messages::intra_backend::{DsFanOutMessage, DsFanOutPayload},
    qs::QsConnector,
    rate_limiter::{RateLimiter, RlConfig, RlKey, provider::RlPostgresStorage},
};

use super::{
    Ds,
    group_state::{DsGroupState, StorableDsGroupData},
};

pub struct GrpcDs<Qep: QsConnector> {
    ds: Ds,
    qs_connector: Qep,
}

const MAX_CONCURRENT_FANOUTS: usize = 128;

impl<Qep: QsConnector> GrpcDs<Qep> {
    pub fn new(ds: Ds, qs_connector: Qep) -> Self {
        Self { ds, qs_connector }
    }

    /// Extract and verify the payload with leaf verifying key from an MLS message.
    ///
    /// Also loads the group data and group state from the database.
    async fn leaf_verify<R, P>(&self, request: R) -> Result<LeafVerificationData<P>, Status>
    where
        R: WithGroupStateEarKey + WithMessage + Verifiable,
        P: VerifiedStruct<R>,
    {
        self.leaf_verify_with_sender(request, None).await
    }

    /// Same as `leaf_verify` but allows to specify the sender index.
    ///
    /// If the sender index is not specified, the sender is extracted from the message.
    async fn leaf_verify_with_sender<R, P>(
        &self,
        request: R,
        sender_index: Option<LeafNodeIndex>,
    ) -> Result<LeafVerificationData<P>, Status>
    where
        R: WithGroupStateEarKey + WithMessage + Verifiable,
        P: VerifiedStruct<R>,
    {
        let ear_key = request.ear_key()?;
        let message = request.message()?;
        let qgid = message.validated_qgid(self.ds.own_domain())?;

        let (group_data, group_state) = self.load_group_state(&qgid, &ear_key).await?;

        // verify signature
        let sender_index = sender_index.map(Ok).unwrap_or_else(|| {
            match *message.sender().ok_or_missing_field("sender")? {
                Sender::Member(sender_index) => Ok(sender_index),
                _ => Err(Status::invalid_argument(
                    "unexpected sender: expected member",
                )),
            }
        })?;

        let verifying_key: LeafVerifyingKeyRef = group_state
            .group()
            .leaf(sender_index)
            .ok_or(Status::invalid_argument("unknown sender"))?
            .signature_key()
            .into();
        let payload: P = request.verify(verifying_key).map_err(InvalidSignature)?;

        Ok(LeafVerificationData {
            ear_key,
            group_data,
            group_state,
            sender_index,
            payload,
            message,
        })
    }

    /// Loads encrypted group state from the database and decrypts it.
    ///
    /// If the group state has expired, the group is deleted and not found is returned.
    async fn load_group_state(
        &self,
        qgid: &QualifiedGroupId,
        ear_key: &GroupStateEarKey,
    ) -> Result<(StorableDsGroupData, DsGroupState), Status> {
        let group_data = StorableDsGroupData::load(&self.ds.db_pool, qgid)
            .await?
            .ok_or(GroupNotFoundError)?;
        if group_data.has_expired() {
            StorableDsGroupData::delete(&self.ds.db_pool, qgid).await?;
            return Err(GroupNotFoundError.into());
        }
        let group_state = DsGroupState::decrypt(&group_data.encrypted_group_state, ear_key)?;
        Ok((group_data, group_state))
    }

    async fn fan_out_message(
        &self,
        mls_message: SerializedMlsMessage,
        destination_clients: impl IntoIterator<Item = identifiers::QsReference>,
    ) -> TimeStamp {
        let queue_message_payload = QsQueueMessagePayload::from(mls_message);
        let timestamp = queue_message_payload.timestamp;
        let fan_out_payload = DsFanOutPayload::QueueMessage(queue_message_payload);

        let mut join_set: JoinSet<Result<(), <Qep as QsConnector>::EnqueueError>> = JoinSet::new();
        for client_reference in destination_clients {
            while MAX_CONCURRENT_FANOUTS <= join_set.len() {
                join_set
                    .join_next()
                    .await
                    .expect("logic error")
                    .map_err(DistributeMessageError::Join)
                    .and_then(|result| result.map_err(DistributeMessageError::Connector))
                    .inspect_err(|error| error!(%error, "Failed to dispatch message"))
                    .ok();
            }
            join_set.spawn(self.qs_connector.dispatch(DsFanOutMessage {
                payload: fan_out_payload.clone(),
                client_reference,
            }));
        }

        while let Some(result) = join_set.join_next().await {
            result
                .map_err(DistributeMessageError::Join)
                .and_then(|result| result.map_err(DistributeMessageError::Connector))
                .inspect_err(|error| error!(%error, "Failed to dispatch message"))
                .ok();
        }

        timestamp
    }

    async fn update_group_data(
        &self,
        mut group_data: StorableDsGroupData,
        group_state: DsGroupState,
        ear_key: &GroupStateEarKey,
    ) -> Result<(), Status> {
        let encrypted_group_state = group_state.encrypt(ear_key)?;
        group_data.encrypted_group_state = encrypted_group_state;
        group_data.update(&self.ds.db_pool).await.map_err(|error| {
            error!(%error, "Failed to update group state");
            Status::internal("Failed to update group state")
        })?;
        Ok(())
    }
}

/// Extracted data in leaf verification
struct LeafVerificationData<P> {
    ear_key: GroupStateEarKey,
    group_data: StorableDsGroupData,
    group_state: DsGroupState,
    sender_index: LeafNodeIndex,
    payload: P,
    message: AssistedMessageIn,
}

#[async_trait]
impl<Qep: QsConnector> DeliveryService for GrpcDs<Qep> {
    async fn request_group_id(
        &self,
        _request: Request<RequestGroupIdRequest>,
    ) -> Result<Response<RequestGroupIdResponse>, Status> {
        let qgid = self.ds.request_group_id().await;
        Ok(Response::new(RequestGroupIdResponse {
            group_id: Some(qgid.ref_into()),
        }))
    }

    async fn create_group(
        &self,
        request: Request<CreateGroupRequest>,
    ) -> Result<Response<CreateGroupResponse>, Status> {
        let request = request.into_inner();

        // TODO: signature verification?
        let payload = request.payload.ok_or_missing_field("payload")?;
        let qgid = payload.validated_qgid(&self.ds.own_domain)?;
        let ear_key = payload.ear_key()?;

        let reserved_group_id = self
            .ds
            .claim_reserved_group_id(qgid.group_uuid())
            .await
            .ok_or_else(|| Status::invalid_argument("unreserved group id"))?;

        // create group
        let group_info: MlsMessageIn = payload
            .group_info
            .as_ref()
            .ok_or_missing_field("group_info")?
            .try_ref_into()
            .invalid_tls("group_info")?;
        let MlsMessageBodyIn::GroupInfo(group_info) = group_info.extract() else {
            return Err(Status::invalid_argument("invalid message"));
        };

        let ratchet_tree: RatchetTreeIn = payload
            .ratchet_tree
            .as_ref()
            .ok_or_missing_field("ratchet_tree")?
            .try_ref_into()
            .invalid_tls("ratchet_tree")?;

        let provider = Provider::default();
        let group = Group::new(&provider, group_info.clone(), ratchet_tree).map_err(|error| {
            error!(%error, "failed to create group");
            Status::internal("failed to create group")
        })?;

        // Extract user id
        let members = group.members().collect::<Vec<_>>();

        let &[own_leaf] = &members.as_slice() else {
            error!(members = %members.len(), "group must have exactly one member");
            return Err(Status::invalid_argument(
                "group must have exactly one member",
            ));
        };

        let credential =
            ClientCredential::tls_deserialize_exact_bytes(own_leaf.credential.serialized_content())
                .map_err(|_| Status::invalid_argument("invalid credential"))?;
        let user_id = credential.identity().uuid();

        // Configure the rate-limiting
        let rl_key = RlKey::new(
            b"ds",
            b"reserve_group_id",
            &[b"user_uuid", user_id.as_bytes()],
        );
        let config = RlConfig {
            max_requests: 100,
            time_window: TimeDelta::hours(1),
        };
        let rl_storage = RlPostgresStorage::new(self.ds.db_pool.clone());
        let rl = RateLimiter::new(config, rl_storage);

        // Apply the rate-limiting
        if !rl.allowed(rl_key).await {
            return Err(Status::resource_exhausted(
                "Too many requests, please try again later",
            ));
        }

        // encrypt and store group state
        let encrypted_user_profile_key = payload
            .encrypted_user_profile_key
            .ok_or_missing_field("encrypted_user_profile_key")?
            .try_into()?;
        let creator_client_reference = payload
            .creator_client_reference
            .ok_or_missing_field("creator_client_reference")?
            .try_into()?;
        let room_state = mimi_room_policy::RoomState::try_from_ref(
            &payload.room_state.ok_or_missing_field("room_state")?,
        )
        .map_err(|_| Status::invalid_argument("Invalid room_state message"))?;

        let room_state = VerifiedRoomState::verify(room_state).map_err(|e| {
            warn!(%e, "proposed room policy failed verification");
            Status::invalid_argument("Room state verification failed")
        })?;

        let group_state = DsGroupState::new(
            provider,
            group,
            encrypted_user_profile_key,
            creator_client_reference,
            room_state,
        );
        let encrypted_group_state = group_state.encrypt(&ear_key)?;

        StorableDsGroupData::new_and_store(
            &self.ds.db_pool,
            reserved_group_id,
            encrypted_group_state,
        )
        .await
        .map_err(|error| {
            error!(%error, "failed to store group state");
            Status::internal("failed to store group state")
        })?;

        Ok(Response::new(CreateGroupResponse {}))
    }

    async fn welcome_info(
        &self,
        request: Request<WelcomeInfoRequest>,
    ) -> Result<Response<WelcomeInfoResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let sender: ClientVerifyingKey = request
            .payload
            .as_ref()
            .ok_or_missing_field("payload")?
            .sender
            .clone()
            .ok_or_missing_field("payload")?
            .into();
        let payload: WelcomeInfoPayload = request.verify(&sender).map_err(InvalidSignature)?;

        let qgid = payload.validated_qgid(&self.ds.own_domain)?;
        let ear_key = payload.ear_key()?;
        let (_, mut group_state) = self.load_group_state(&qgid, &ear_key).await?;

        let welcome_info_params = WelcomeInfoParams {
            sender: sender.clone(),
            epoch: payload.epoch.ok_or_missing_field("epoch")?.into(),
            group_id: qgid.into(),
        };
        let ratchet_tree = group_state
            .welcome_info(welcome_info_params)
            .ok_or(NoWelcomeInfoFound)?;
        Ok(Response::new(WelcomeInfoResponse {
            ratchet_tree: Some(ratchet_tree.try_ref_into().invalid_tls("ratchet_tree")?),
            encrypted_user_profile_keys: group_state
                .encrypted_user_profile_keys()
                .into_iter()
                .map(From::from)
                .collect(),
            room_state: Some(
                group_state
                    .room_state
                    .unverified()
                    .try_ref_into()
                    .invalid_tls("room_state")?,
            ),
        }))
    }

    async fn external_commit_info(
        &self,
        request: Request<ExternalCommitInfoRequest>,
    ) -> Result<Response<ExternalCommitInfoResponse>, Status> {
        let request = request.into_inner();

        let qgid = request.qgid.ok_or_missing_field("qgid")?.try_ref_into()?;
        let ear_key = request
            .group_state_ear_key
            .ok_or_missing_field("group_state_ear_key")?
            .try_ref_into()?;

        let (_, group_state) = self.load_group_state(&qgid, &ear_key).await?;

        let commit_info = group_state.external_commit_info();

        Ok(Response::new(ExternalCommitInfoResponse {
            group_info: Some(
                commit_info
                    .group_info
                    .try_into()
                    .invalid_tls("group_info")?,
            ),
            ratchet_tree: Some(
                commit_info
                    .ratchet_tree
                    .try_ref_into()
                    .invalid_tls("ratchet_tree")?,
            ),
            encrypted_user_profile_keys: commit_info
                .encrypted_user_profile_keys
                .into_iter()
                .map(From::from)
                .collect(),
            room_state: Some(
                commit_info
                    .room_state
                    .unverified()
                    .try_ref_into()
                    .invalid_tls("room_state")?,
            ),
        }))
    }

    async fn connection_group_info(
        &self,
        request: Request<ConnectionGroupInfoRequest>,
    ) -> Result<Response<ConnectionGroupInfoResponse>, Status> {
        let request = request.into_inner();

        let qgid: QualifiedGroupId = request
            .group_id
            .ok_or_missing_field("group_id")?
            .try_ref_into()?;
        let ear_key: GroupStateEarKey = request
            .group_state_ear_key
            .ok_or_missing_field("group_state_ear_key")?
            .try_ref_into()?;

        let (_, group_state) = self.load_group_state(&qgid, &ear_key).await?;
        let commit_info = group_state.external_commit_info();

        let group_info = commit_info
            .group_info
            .try_into()
            .invalid_tls("group_info")?;
        let ratchet_tree = commit_info
            .ratchet_tree
            .try_ref_into()
            .invalid_tls("ratchet_tree")?;
        Ok(Response::new(ConnectionGroupInfoResponse {
            group_info: Some(group_info),
            ratchet_tree: Some(ratchet_tree),
            encrypted_user_profile_keys: commit_info
                .encrypted_user_profile_keys
                .into_iter()
                .map(From::from)
                .collect(),
            room_state: Some(
                commit_info
                    .room_state
                    .unverified()
                    .try_ref_into()
                    .invalid_tls("room_state")?,
            ),
        }))
    }

    async fn join_connection_group(
        &self,
        request: Request<JoinConnectionGroupRequest>,
    ) -> Result<Response<JoinConnectionGroupResponse>, Status> {
        let request = request.into_inner();

        let external_commit: AssistedMessageIn = request
            .external_commit
            .ok_or_missing_field("external_commit")?
            .try_ref_into()
            .invalid_tls("external_commit")?;
        let qgid = external_commit.validated_qgid(self.ds.own_domain())?;
        let ear_key = request
            .group_state_ear_key
            .ok_or_missing_field("group_state_ear_key")?
            .try_ref_into()?;

        let (group_data, mut group_state) = self.load_group_state(&qgid, &ear_key).await?;

        let params = JoinConnectionGroupParams {
            external_commit,
            qs_client_reference: request
                .qs_client_reference
                .ok_or_missing_field("qs_client_reference")?
                .try_into()?,
        };

        let destination_clients: Vec<_> = group_state.destination_clients().collect();
        let group_message = group_state.join_connection_group(params)?;

        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        let timestamp = self
            .fan_out_message(group_message, destination_clients)
            .await;

        Ok(Response::new(JoinConnectionGroupResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn resync(
        &self,
        request: Request<ResyncRequest>,
    ) -> Result<Response<ResyncResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let LeafVerificationData {
            ear_key,
            group_data,
            mut group_state,
            sender_index,
            message: external_commit,
            ..
        } = self.leaf_verify::<_, ResyncPayload>(request).await?;

        let destination_clients: Vec<_> = group_state
            .other_destination_clients(sender_index)
            .collect();

        let group_message = group_state.resync_client(external_commit, sender_index)?;
        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        let timestamp = self
            .fan_out_message(group_message, destination_clients)
            .await;

        Ok(Response::new(ResyncResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn self_remove(
        &self,
        request: Request<SelfRemoveRequest>,
    ) -> Result<Response<SelfRemoveResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let LeafVerificationData {
            ear_key,
            group_data,
            mut group_state,
            sender_index,
            message: remove_proposal,
            ..
        } = self.leaf_verify::<_, SelfRemovePayload>(request).await?;

        let destination_clients: Vec<_> = group_state
            .other_destination_clients(sender_index)
            .collect();

        let group_message = group_state.self_remove_client(remove_proposal)?;
        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        let timestamp = self
            .fan_out_message(group_message, destination_clients)
            .await;

        Ok(Response::new(SelfRemoveResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let sender_index: LeafNodeIndex = request
            .payload
            .as_ref()
            .ok_or_missing_field("sender")?
            .sender
            .ok_or_missing_field("sender")?
            .into();

        let LeafVerificationData {
            group_state,
            message: mls_message,
            ..
        } = self
            .leaf_verify_with_sender::<_, SendMessagePayload>(request, Some(sender_index))
            .await?;

        let destination_clients = group_state.other_destination_clients(sender_index);

        let timestamp = self
            .fan_out_message(
                mls_message.into_serialized_mls_message(),
                destination_clients,
            )
            .await;

        Ok(Response::new(SendMessageResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn delete_group(
        &self,
        request: Request<DeleteGroupRequest>,
    ) -> Result<Response<DeleteGroupResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let LeafVerificationData {
            ear_key,
            group_data,
            mut group_state,
            sender_index,
            message: commit,
            ..
        } = self.leaf_verify::<_, DeleteGroupPayload>(request).await?;

        let destination_clients: Vec<_> = group_state
            .other_destination_clients(sender_index)
            .collect();

        let group_message = group_state.delete_group(commit)?;

        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        let timestamp = self
            .fan_out_message(group_message, destination_clients)
            .await;

        Ok(Response::new(DeleteGroupResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn group_operation(
        &self,
        request: Request<GroupOperationRequest>,
    ) -> Result<Response<GroupOperationResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let LeafVerificationData {
            ear_key,
            group_data,
            mut group_state,
            sender_index,
            payload,
            message: commit,
            ..
        }: LeafVerificationData<GroupOperationPayload> = self.leaf_verify(request).await?;

        let params = GroupOperationParams {
            commit,
            add_users_info_option: payload
                .add_users_info
                .map(|info| info.try_into())
                .transpose()?,
        };

        let destination_clients: Vec<_> = group_state
            .other_destination_clients(sender_index)
            .collect();

        let (group_message, welcome_bundles) =
            group_state.group_operation(params, &ear_key).await?;

        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        let timestamp = self
            .fan_out_message(group_message, destination_clients)
            .await;

        for message in welcome_bundles {
            self.qs_connector
                .dispatch(message)
                .await
                .map_err(DistributeMessageError::Connector)?;
        }

        Ok(Response::new(GroupOperationResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn update_profile_key(
        &self,
        request: Request<UpdateProfileKeyRequest>,
    ) -> Result<Response<UpdateProfileKeyResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        let ear_key = request.ear_key()?;
        let qgid = payload.validated_qgid(self.ds.own_domain())?;
        let sender_index = payload.sender.ok_or_missing_field("sender")?.into();

        let (group_data, mut group_state) = self.load_group_state(&qgid, &ear_key).await?;

        // verify signature
        let verifying_key: LeafVerifyingKeyRef = group_state
            .group()
            .leaf(sender_index)
            .ok_or_else(|| Status::invalid_argument("unknown sender"))?
            .signature_key()
            .into();
        let payload: UpdateProfileKeyPayload =
            request.verify(verifying_key).map_err(InvalidSignature)?;

        let user_profile_key = payload
            .encrypted_user_profile_key
            .ok_or_missing_field("user_profile_key")?
            .try_into()?;
        let params = UserProfileKeyUpdateParams {
            group_id: qgid.into(),
            sender_index,
            user_profile_key,
        };
        let fan_out_payload = DsFanOutPayload::QueueMessage(
            QsQueueMessagePayload::try_from(&params).tls_failed("QsQueueMessagePayload")?,
        );

        let destination_clients: Vec<_> = group_state
            .other_destination_clients(sender_index)
            .collect();

        group_state.update_user_profile_key(sender_index, params.user_profile_key)?;

        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        for client_reference in destination_clients {
            self.qs_connector
                .dispatch(DsFanOutMessage {
                    payload: fan_out_payload.clone(),
                    client_reference,
                })
                .await
                .map_err(DistributeMessageError::Connector)?;
        }

        Ok(Response::new(UpdateProfileKeyResponse {}))
    }

    async fn provision_attachment(
        &self,
        request: Request<ProvisionAttachmentRequest>,
    ) -> Result<Response<ProvisionAttachmentResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        let ear_key = request.ear_key()?;
        let qgid = payload.validated_qgid(self.ds.own_domain())?;
        let sender_index = payload.sender.ok_or_missing_field("sender")?.into();

        let (_group_data, group_state) = self.load_group_state(&qgid, &ear_key).await?;

        // verify signature
        let verifying_key: LeafVerifyingKeyRef = group_state
            .group()
            .leaf(sender_index)
            .ok_or_else(|| Status::invalid_argument("unknown sender"))?
            .signature_key()
            .into();
        let payload = request.verify(verifying_key).map_err(InvalidSignature)?;

        Ok(self.ds.provision_attachment(payload).await?)
    }

    async fn get_attachment_url(
        &self,
        request: Request<GetAttachmentUrlRequest>,
    ) -> Result<Response<GetAttachmentUrlResponse>, Status> {
        let request = request.into_inner();

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;

        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        let ear_key = request.ear_key()?;
        let qgid = payload.validated_qgid(self.ds.own_domain())?;
        let sender_index = payload.sender.ok_or_missing_field("sender")?.into();

        let (_group_data, group_state) = self.load_group_state(&qgid, &ear_key).await?;

        // verify signature
        let verifying_key: LeafVerifyingKeyRef = group_state
            .group()
            .leaf(sender_index)
            .ok_or_else(|| Status::invalid_argument("unknown sender"))?
            .signature_key()
            .into();
        let payload: GetAttachmentUrlPayload =
            request.verify(verifying_key).map_err(InvalidSignature)?;

        let attachment_id = payload
            .attachment_id
            .ok_or_missing_field("attachment_id")?
            .into();
        let attachment_id = AttachmentId::new(attachment_id);

        Ok(self.ds.get_attachment_url(attachment_id).await?)
    }
}

#[derive(Debug, Error)]
enum DistributeMessageError<E> {
    #[error(transparent)]
    Join(JoinError),
    #[error(transparent)]
    Connector(E),
}

impl<E: std::error::Error> From<DistributeMessageError<E>> for Status {
    fn from(error: DistributeMessageError<E>) -> Self {
        error!(%error, "Failed to distribute message");
        Status::internal("failed to distribute message")
    }
}

struct GroupNotFoundError;

impl From<GroupNotFoundError> for Status {
    fn from(_: GroupNotFoundError) -> Self {
        Status::not_found("group not found")
    }
}

struct InvalidSignature(SignatureVerificationError);

impl From<InvalidSignature> for Status {
    fn from(e: InvalidSignature) -> Self {
        error!(error =% e.0, "invalid signature");
        Status::unauthenticated("invalid signature")
    }
}

/// Protobuf containing a qualified group id
trait WithQualifiedGroupId {
    fn qgid(&self) -> Result<QualifiedGroupId, Status>;

    fn validated_qgid(&self, own_domain: &Fqdn) -> Result<QualifiedGroupId, Status> {
        let qgid = self.qgid()?;
        if qgid.owning_domain() == own_domain {
            Ok(qgid)
        } else {
            Err(NonMatchingOwnDomain(qgid).into())
        }
    }
}

struct NonMatchingOwnDomain(QualifiedGroupId);

impl From<NonMatchingOwnDomain> for Status {
    fn from(e: NonMatchingOwnDomain) -> Self {
        error!(qgid =% e.0, "group id domain does not match own domain");
        Status::invalid_argument("group id domain does not match own domain")
    }
}

impl WithQualifiedGroupId for AssistedMessageIn {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.group_id()
            .try_into()
            .invalid_tls("group_id")
            .map_err(From::from)
    }
}

impl WithQualifiedGroupId for CreateGroupPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.qgid
            .as_ref()
            .ok_or_missing_field("qgid")?
            .try_ref_into()
            .map_err(From::from)
    }
}

impl WithQualifiedGroupId for WelcomeInfoPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.qgid
            .as_ref()
            .ok_or_missing_field("qgid")?
            .try_ref_into()
            .map_err(From::from)
    }
}

impl WithQualifiedGroupId for UpdateProfileKeyPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.group_id
            .as_ref()
            .ok_or_missing_field("group_id")?
            .try_ref_into()
            .map_err(From::from)
    }
}

impl WithQualifiedGroupId for ProvisionAttachmentPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.group_id
            .as_ref()
            .ok_or_missing_field("group_id")?
            .try_ref_into()
            .map_err(From::from)
    }
}

impl WithQualifiedGroupId for GetAttachmentUrlPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.group_id
            .as_ref()
            .ok_or_missing_field("group_id")?
            .try_ref_into()
            .map_err(From::from)
    }
}

/// Protobuf containing a group state ear key
trait WithGroupStateEarKey {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey>;

    fn ear_key(&self) -> Result<GroupStateEarKey, Status> {
        self.ear_key_proto()
            .ok_or_missing_field("group_state_ear_key")?
            .try_ref_into()
            .map_err(From::from)
    }
}

impl WithGroupStateEarKey for SendMessageRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for CreateGroupPayload {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for DeleteGroupRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for GroupOperationRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for SelfRemoveRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for WelcomeInfoPayload {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for ResyncRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for UpdateProfileKeyRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for ProvisionAttachmentRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

impl WithGroupStateEarKey for GetAttachmentUrlRequest {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.payload.as_ref()?.group_state_ear_key.as_ref()
    }
}

/// Request containing an MLS message
trait WithMessage {
    fn message(&self) -> Result<AssistedMessageIn, Status>;
}

impl WithMessage for SendMessageRequest {
    fn message(&self) -> Result<AssistedMessageIn, Status> {
        let payload = self.payload.as_ref().ok_or_missing_field("payload")?;
        let message = payload.message.as_ref().ok_or_missing_field("message")?;
        let message = message.try_ref_into().invalid_tls("message")?;
        Ok(message)
    }
}

impl WithMessage for GroupOperationRequest {
    fn message(&self) -> Result<AssistedMessageIn, Status> {
        let payload = self.payload.as_ref().ok_or_missing_field("payload")?;
        let commit = payload.commit.as_ref().ok_or_missing_field("commit")?;
        let commit = commit.try_ref_into().invalid_tls("commit")?;
        Ok(commit)
    }
}

impl WithMessage for DeleteGroupRequest {
    fn message(&self) -> Result<AssistedMessageIn, Status> {
        let payload = self.payload.as_ref().ok_or_missing_field("payload")?;
        let commit = payload.commit.as_ref().ok_or_missing_field("commit")?;
        let commit = commit.try_ref_into().invalid_tls("commit")?;
        Ok(commit)
    }
}

impl WithMessage for SelfRemoveRequest {
    fn message(&self) -> Result<AssistedMessageIn, Status> {
        let payload = self.payload.as_ref().ok_or_missing_field("payload")?;
        let remove_proposal = payload
            .remove_proposal
            .as_ref()
            .ok_or_missing_field("remove_proposal")?;
        let remove_proposal = remove_proposal
            .try_ref_into()
            .invalid_tls("remove_proposal")?;
        Ok(remove_proposal)
    }
}

impl WithMessage for ResyncRequest {
    fn message(&self) -> Result<AssistedMessageIn, Status> {
        let payload = self.payload.as_ref().ok_or_missing_field("payload")?;
        let external_commit = payload
            .external_commit
            .as_ref()
            .ok_or_missing_field("external_commit")?;
        let message = external_commit
            .try_ref_into()
            .invalid_tls("external_commit")?;
        Ok(message)
    }
}

struct NoWelcomeInfoFound;

impl From<NoWelcomeInfoFound> for Status {
    fn from(_: NoWelcomeInfoFound) -> Self {
        Status::not_found("no welcome info found")
    }
}
