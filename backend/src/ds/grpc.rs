// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::Group,
    messages::{AssistedMessageIn, SerializedMlsMessage},
    openmls::prelude::{LeafNodeIndex, MlsMessageBodyIn, MlsMessageIn, RatchetTreeIn, Sender},
};
use phnxprotos::{
    convert::{RefInto, TryRefInto},
    delivery_service::v1::{self, *},
    validation::{InvalidTlsExt, MissingFieldExt},
};
use phnxtypes::{crypto::signatures::signable::Verifiable, identifiers, time::TimeStamp};
use phnxtypes::{
    crypto::{
        ear::keys::GroupStateEarKey,
        signatures::{keys::LeafVerifyingKey, traits::SignatureVerificationError},
    },
    identifiers::{Fqdn, QualifiedGroupId},
    messages::client_ds::QsQueueMessagePayload,
};
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use crate::{
    ds::process::Provider,
    messages::intra_backend::{DsFanOutMessage, DsFanOutPayload},
    qs::QsConnector,
};

use super::{
    Ds,
    group_state::{DsGroupState, StorableDsGroupData},
};

pub struct GrpcDs<Qep: QsConnector> {
    ds: Ds,
    qs_connector: Qep,
}

impl<Qep: QsConnector> GrpcDs<Qep> {
    pub fn new(ds: Ds, qs_connector: Qep) -> Self {
        Self { ds, qs_connector }
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
        destination_clients: Vec<identifiers::QsReference>,
    ) -> Result<TimeStamp, tonic::Status> {
        let queue_message_payload = QsQueueMessagePayload::from(mls_message);
        let timestamp = queue_message_payload.timestamp;
        let fan_out_payload = DsFanOutPayload::QueueMessage(queue_message_payload);

        for client_reference in destination_clients {
            self.qs_connector
                .dispatch(DsFanOutMessage {
                    payload: fan_out_payload.clone(),
                    client_reference,
                })
                .await
                .map_err(DistributeMessageError)?;
        }

        Ok(timestamp)
    }

    async fn update_group_data(
        &self,
        mut group_data: StorableDsGroupData,
        group_state: DsGroupState,
        ear_key: &GroupStateEarKey,
    ) -> Result<(), tonic::Status> {
        let encrypted_group_state = group_state.encrypt(ear_key)?;
        group_data.encrypted_group_state = encrypted_group_state;
        group_data.update(&self.ds.db_pool).await.map_err(|error| {
            error!(%error, "Failed to update group state");
            Status::internal("Failed to update group state")
        })?;
        Ok(())
    }
}

#[async_trait]
impl<Qep: QsConnector> v1::delivery_service_server::DeliveryService for GrpcDs<Qep> {
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

        // encrypt and store group state
        let encrypted_identity_link_key = payload
            .encrypted_identity_link_key
            .ok_or_missing_field("encrypted_identity_link_key")?
            .try_into()?;
        let encrypted_user_profile_key = payload
            .encrypted_user_profile_key
            .ok_or_missing_field("encrypted_user_profile_key")?
            .try_into()?;
        let creator_client_reference = payload
            .creator_client_reference
            .as_ref()
            .ok_or_missing_field("creator_client_reference")?
            .try_ref_into()?;
        let group_state = DsGroupState::new(
            provider,
            group,
            encrypted_identity_link_key,
            encrypted_user_profile_key,
            creator_client_reference,
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
        _request: Request<WelcomeInfoRequest>,
    ) -> Result<Response<WelcomeInfoResponse>, Status> {
        todo!()
    }

    async fn external_commit_info(
        &self,
        _request: Request<ExternalCommitInfoRequest>,
    ) -> Result<Response<ExternalCommitInfoResponse>, Status> {
        todo!()
    }

    async fn connection_group_info(
        &self,
        _request: Request<ConnectionGroupInfoRequest>,
    ) -> Result<Response<ConnectionGroupInfoResponse>, Status> {
        todo!()
    }

    async fn update_qs_client_reference(
        &self,
        _request: Request<UpdateQsClientReferenceRequest>,
    ) -> Result<Response<UpdateQsClientReferenceResponse>, Status> {
        todo!()
    }

    async fn update(
        &self,
        _request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        todo!()
    }

    async fn join_connection_group(
        &self,
        _request: Request<JoinConnectionGroupRequest>,
    ) -> Result<Response<JoinConnectionGroupResponse>, Status> {
        todo!()
    }

    async fn resync(
        &self,
        _request: Request<ResyncRequest>,
    ) -> Result<Response<ResyncResponse>, Status> {
        todo!()
    }

    async fn self_remove(
        &self,
        _request: Request<SelfRemoveRequest>,
    ) -> Result<Response<SelfRemoveResponse>, Status> {
        todo!()
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
        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        let mls_message: AssistedMessageIn = payload
            .message
            .as_ref()
            .ok_or_missing_field("message")?
            .try_ref_into()
            .invalid_tls("message")?;

        let qgid = mls_message.validated_qgid(self.ds.own_domain())?;
        let ear_key = payload.ear_key()?;
        let (_, group_state) = self.load_group_state(&qgid, &ear_key).await?;

        // verify
        let sender_index: LeafNodeIndex = payload.sender.ok_or_missing_field("sender")?.into();
        let verifying_key: LeafVerifyingKey = group_state
            .group()
            .leaf(sender_index)
            .ok_or(UnknownSenderError(sender_index))?
            .signature_key()
            .into();
        let _: SendMessagePayload = request.verify(&verifying_key).map_err(InvalidSignature)?;

        let destination_clients: Vec<_> = group_state
            .member_profiles
            .iter()
            .filter_map(|(client_index, client_profile)| {
                if client_index == &sender_index {
                    None
                } else {
                    Some(client_profile.client_queue_config.clone())
                }
            })
            .collect();

        let timestamp = self
            .fan_out_message(
                mls_message.into_serialized_mls_message(),
                destination_clients,
            )
            .await?;

        Ok(Response::new(SendMessageResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn delete_group(
        &self,
        request: Request<DeleteGroupRequest>,
    ) -> Result<Response<DeleteGroupResponse>, Status> {
        let inner = request.into_inner();
        let request = inner;

        request
            .signature
            .as_ref()
            .ok_or_missing_field("signature")?;
        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        let commit: AssistedMessageIn = payload
            .commit
            .as_ref()
            .ok_or_missing_field("message")?
            .try_ref_into()
            .invalid_tls("message")?;

        let qgid = commit.validated_qgid(self.ds.own_domain())?;
        let ear_key = payload.ear_key()?;
        let (group_data, mut group_state) = self.load_group_state(&qgid, &ear_key).await?;

        // verify
        let Sender::Member(sender_index) = *commit.sender().ok_or_missing_field("commit.sender")?
        else {
            return Err(Status::invalid_argument(
                "expected a member sender in commit",
            ));
        };
        let verifying_key: LeafVerifyingKey = group_state
            .group()
            .leaf(sender_index)
            .ok_or(UnknownSenderError(sender_index))?
            .signature_key()
            .into();
        let _: DeleteGroupPayload = request.verify(&verifying_key).map_err(InvalidSignature)?;

        let destination_clients: Vec<_> = group_state
            .member_profiles
            .iter()
            .filter_map(|(client_index, client_profile)| {
                if client_index == &sender_index {
                    None
                } else {
                    Some(client_profile.client_queue_config.clone())
                }
            })
            .collect();

        let group_message = group_state.delete_group(commit).map_err(|error| {
            error!(%error, "Failed to delete group");
            Status::internal("Failed to delete group")
        })?;

        self.update_group_data(group_data, group_state, &ear_key)
            .await?;

        let timestamp = self
            .fan_out_message(group_message, destination_clients)
            .await?;

        Ok(DeleteGroupResponse {
            fanout_timestamp: Some(timestamp.into()),
        }
        .into())
    }

    async fn group_operation(
        &self,
        _request: Request<GroupOperationRequest>,
    ) -> Result<Response<GroupOperationResponse>, Status> {
        todo!()
    }
}

struct DistributeMessageError<E>(E);

impl<E: std::error::Error> From<DistributeMessageError<E>> for Status {
    fn from(e: DistributeMessageError<E>) -> Self {
        error!(error =% e.0, "Failed to distribute message");
        Status::internal("Failed to distribute message")
    }
}

struct GroupNotFoundError;

impl From<GroupNotFoundError> for Status {
    fn from(_: GroupNotFoundError) -> Self {
        Status::not_found("Group not found")
    }
}

struct UnknownSenderError(LeafNodeIndex);

impl From<UnknownSenderError> for Status {
    fn from(e: UnknownSenderError) -> Self {
        error!("Could not find leaf with index {}", e.0);
        Status::invalid_argument("Unknown sender")
    }
}

struct InvalidSignature(SignatureVerificationError);

impl From<InvalidSignature> for Status {
    fn from(e: InvalidSignature) -> Self {
        error!(error =% e.0, "Invalid signature");
        Status::unauthenticated("Invalid signature")
    }
}

/// Extension trait for extracting and validating a fully qualified group id from a protobuf
/// message
trait QualifiedGroupIdExt {
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
        error!(qgid =% e.0, "Group id domain does not match own domain");
        Status::invalid_argument("Group id domain does not match own domain")
    }
}

impl QualifiedGroupIdExt for AssistedMessageIn {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.group_id()
            .try_into()
            .invalid_tls("group_id")
            .map_err(From::from)
    }
}

impl QualifiedGroupIdExt for CreateGroupPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.qgid
            .as_ref()
            .ok_or_missing_field("qgid")?
            .try_ref_into()
            .map_err(From::from)
    }
}

/// Extension trait for extracting the group state ear key from a protobuf message
trait GroupStateEarKeyExt {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey>;

    fn ear_key(&self) -> Result<GroupStateEarKey, Status> {
        self.ear_key_proto()
            .ok_or_missing_field("group_state_ear_key")?
            .try_ref_into()
            .map_err(From::from)
    }
}

impl GroupStateEarKeyExt for SendMessagePayload {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

impl GroupStateEarKeyExt for CreateGroupPayload {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

impl GroupStateEarKeyExt for DeleteGroupPayload {
    fn ear_key_proto(&self) -> Option<&v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}
