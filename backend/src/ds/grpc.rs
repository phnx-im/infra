#![allow(unused_variables)]

use mls_assist::messages::AssistedMessageIn;
use openmls::prelude::{LeafNodeIndex, SignaturePublicKey};
use phnxtypes::{
    crypto::{
        ear::{
            Ciphertext,
            keys::{EncryptedIdentityLinkKey, GroupStateEarKey},
        },
        signatures::{
            keys::LeafVerifyingKey, signable::Verifiable, traits::SignatureVerificationError,
        },
    },
    identifiers::{Fqdn, QualifiedGroupId},
    messages::client_ds::QsQueueMessagePayload,
};
use protos::{
    common::convert::{CiphertextError, QualifiedGroupIdError},
    delivery_service::{
        convert::{InvalidGroupStateEarKeyLength, QsReferenceError},
        v1::{
            ConnectionGroupInfoRequest, ConnectionGroupInfoResponse, CreateGroupRequest,
            CreateGroupResponse, DeleteGroupRequest, DeleteGroupResponse,
            ExternalCommitInfoRequest, ExternalCommitInfoResponse, GroupOperationRequest,
            GroupOperationResponse, JoinConnectionGroupRequest, JoinConnectionGroupResponse,
            RequestGroupIdRequest, RequestGroupIdResponse, ResyncRequest, ResyncResponse,
            SelfRemoveRequest, SelfRemoveResponse, SendMessagePayload, SendMessageRequest,
            SendMessageResponse, UpdateQsClientReferenceRequest, UpdateQsClientReferenceResponse,
            UpdateRequest, UpdateResponse, WelcomeInfoPayload, WelcomeInfoRequest,
            WelcomeInfoResponse,
        },
    },
};
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use crate::{
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
}

#[async_trait]
impl<Qep: QsConnector> protos::delivery_service::v1::delivery_service_server::DeliveryService
    for GrpcDs<Qep>
{
    async fn request_group_id(
        &self,
        _request: Request<RequestGroupIdRequest>,
    ) -> Result<Response<RequestGroupIdResponse>, Status> {
        let group_id = self.ds.request_group_id().await;
        Ok(Response::new(RequestGroupIdResponse {
            group_id: Some(group_id.into()),
        }))
    }

    async fn create_group(
        &self,
        request: Request<CreateGroupRequest>,
    ) -> Result<Response<CreateGroupResponse>, Status> {
        let message = request.into_inner();

        let qgid = message.validated_qgid(self.ds.own_domain())?;
        let ear_key = message.ear_key()?;

        let leaf_node = message
            .ratchet_tree
            .ok_or_missing_field("ratchet_tree")?
            .try_into()
            .invalid_tls("ratchet_tree")?;

        let ciphertext: Ciphertext = message
            .encrypted_identity_link_key
            .ok_or_missing_field("encrypted_identity_link_key")?
            .ciphertext
            .ok_or_missing_field("encrypted_identity_link_key.ciphertext")?
            .try_into()
            .map_err(InvalidCiphertext)?;
        let encrypted_identity_link_key = EncryptedIdentityLinkKey::from(ciphertext);

        let creator_qs_reference = message
            .creator_client_reference
            .ok_or_missing_field("creator_client_reference")?
            .try_into()
            .map_err(InvalidQsReference)?;

        let group_info = message
            .group_info
            .ok_or_missing_field("group_info")?
            .try_into()
            .invalid_tls("group_info")?;

        let (reserved_group_id, group_state) = self
            .ds
            .create_group(
                &qgid,
                leaf_node,
                encrypted_identity_link_key,
                creator_qs_reference,
                group_info,
            )
            .await
            .map_err(|error| {
                error!(%error, "Failed to create group");
                Status::internal("Failed to create group")
            })?;

        let encrypted_group_state = group_state.encrypt(&ear_key)?;
        StorableDsGroupData::new_and_store(
            self.ds.pool(),
            reserved_group_id,
            encrypted_group_state,
        )
        .await?;

        Ok(CreateGroupResponse {}.into())
    }

    async fn welcome_info(
        &self,
        request: Request<WelcomeInfoRequest>,
    ) -> Result<Response<WelcomeInfoResponse>, Status> {
        let request = request.into_inner();

        let signature = request.signature.clone().ok_or_missing_field("signature")?;
        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        // verify
        let sender: SignaturePublicKey =
            payload.sender.clone().ok_or_missing_field("sender")?.into();
        let signature_key: LeafVerifyingKey = (&sender).into();
        let payload: WelcomeInfoPayload =
            request.verify(&signature_key).map_err(InvalidSignature)?;

        let ear_key = payload.ear_key()?;
        let qgid = payload.validated_qgid(self.ds.own_domain())?;
        let (mut group_data, mut group_state) = self.load_group_state(&qgid, &ear_key).await?;

        let epoch = payload.epoch.ok_or_missing_field("epoch")?.into();
        let ratchet_tree = group_state
            .group_mut()
            .past_group_state(&epoch, &sender)
            .ok_or_else(|| Status::not_found("No welcome info found"))?
            .clone();

        group_data.encrypted_group_state = group_state.encrypt(&ear_key)?;
        group_data.update(&self.ds.db_pool).await?;

        let response = WelcomeInfoResponse {
            ratchet_tree: Some(ratchet_tree.try_into().tls_failed("ratchet_tree")?),
        };
        Ok(Response::new(response))
    }

    async fn external_commit_info(
        &self,
        request: Request<ExternalCommitInfoRequest>,
    ) -> Result<Response<ExternalCommitInfoResponse>, Status> {
        todo!()
    }

    async fn connection_group_info(
        &self,
        request: Request<ConnectionGroupInfoRequest>,
    ) -> Result<Response<ConnectionGroupInfoResponse>, Status> {
        todo!()
    }

    async fn update_qs_client_reference(
        &self,
        request: Request<UpdateQsClientReferenceRequest>,
    ) -> Result<Response<UpdateQsClientReferenceResponse>, Status> {
        todo!()
    }

    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        todo!()
    }

    async fn join_connection_group(
        &self,
        request: Request<JoinConnectionGroupRequest>,
    ) -> Result<Response<JoinConnectionGroupResponse>, Status> {
        todo!()
    }

    async fn resync(
        &self,
        request: Request<ResyncRequest>,
    ) -> Result<Response<ResyncResponse>, Status> {
        todo!()
    }

    async fn self_remove(
        &self,
        request: Request<SelfRemoveRequest>,
    ) -> Result<Response<SelfRemoveResponse>, Status> {
        todo!()
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let request = request.into_inner();

        let signature = request.signature.clone().ok_or_missing_field("signature")?;
        let payload = request.payload.as_ref().ok_or_missing_field("payload")?;

        let mls_message: AssistedMessageIn = payload
            .message
            .as_ref()
            .ok_or_missing_field("message")?
            .try_into()
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

        let group_message = mls_message.into_serialized_mls_message();
        let queue_message_payload = QsQueueMessagePayload::from(group_message);
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

        Ok(Response::new(SendMessageResponse {
            fanout_timestamp: Some(timestamp.into()),
        }))
    }

    async fn delete_group(
        &self,
        request: Request<DeleteGroupRequest>,
    ) -> Result<Response<DeleteGroupResponse>, Status> {
        todo!()
    }

    async fn group_operation(
        &self,
        request: Request<GroupOperationRequest>,
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

#[derive(Debug, derive_more::Display)]
struct MissingFieldError {
    field_name: &'static str,
}

impl From<MissingFieldError> for Status {
    fn from(e: MissingFieldError) -> Status {
        Status::invalid_argument(format!("Missing field: {}", e.field_name))
    }
}

trait MissingFieldExt {
    type Value;

    fn ok_or_missing_field(
        self,
        field_name: &'static str,
    ) -> Result<Self::Value, MissingFieldError>;
}

impl<T> MissingFieldExt for Option<T> {
    type Value = T;

    fn ok_or_missing_field(self, field_name: &'static str) -> Result<T, MissingFieldError> {
        self.ok_or(MissingFieldError { field_name })
    }
}

struct InvalidTls {
    error: tls_codec::Error,
    field_name: &'static str,
}

impl From<InvalidTls> for Status {
    fn from(e: InvalidTls) -> Status {
        error!(%e.error, "Invalid TLS");
        Status::invalid_argument(format!("Invalid TLS: {}", e.field_name))
    }
}

struct TlsFailed {
    error: tls_codec::Error,
    field_name: &'static str,
}

impl From<TlsFailed> for Status {
    fn from(e: TlsFailed) -> Status {
        error!(%e.error, "TLS serialization failed");
        Status::internal(format!("TLS serialization failed: {}", e.field_name))
    }
}

trait InvalidTlsExt {
    type Value;

    fn invalid_tls(self, field_name: &'static str) -> Result<Self::Value, InvalidTls>;

    fn tls_failed(self, field_name: &'static str) -> Result<Self::Value, TlsFailed>;
}

impl<T> InvalidTlsExt for Result<T, tls_codec::Error> {
    type Value = T;

    fn invalid_tls(self, field_name: &'static str) -> Result<T, InvalidTls> {
        self.map_err(|error| InvalidTls { error, field_name })
    }

    fn tls_failed(self, field_name: &'static str) -> Result<T, TlsFailed> {
        self.map_err(|error| TlsFailed { error, field_name })
    }
}

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

impl QualifiedGroupIdExt for WelcomeInfoPayload {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.qgid
            .as_ref()
            .ok_or_missing_field("qgid")?
            .try_into()
            .map_err(InvalidQualifiedGroupId)
            .map_err(From::from)
    }
}

impl QualifiedGroupIdExt for CreateGroupRequest {
    fn qgid(&self) -> Result<QualifiedGroupId, Status> {
        self.qgid
            .as_ref()
            .ok_or_missing_field("qgid")?
            .try_into()
            .map_err(InvalidQualifiedGroupId)
            .map_err(From::from)
    }
}

struct InvalidQualifiedGroupId(QualifiedGroupIdError);

impl From<InvalidQualifiedGroupId> for Status {
    fn from(e: InvalidQualifiedGroupId) -> Self {
        error!(error =% e.0, "Invalid qualified group id");
        Status::invalid_argument("Invalid qualified group id")
    }
}

trait GroupStateEarKeyExt {
    fn ear_key_proto(&self) -> Option<&protos::delivery_service::v1::GroupStateEarKey>;

    fn ear_key(&self) -> Result<GroupStateEarKey, Status> {
        self.ear_key_proto()
            .ok_or_missing_field("group_state_ear_key")?
            .try_into()
            .map_err(InvalidGroupStateEarKey)
            .map_err(From::from)
    }
}

impl GroupStateEarKeyExt for CreateGroupRequest {
    fn ear_key_proto(&self) -> Option<&protos::delivery_service::v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

impl GroupStateEarKeyExt for WelcomeInfoPayload {
    fn ear_key_proto(&self) -> Option<&protos::delivery_service::v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

impl GroupStateEarKeyExt for SendMessagePayload {
    fn ear_key_proto(&self) -> Option<&protos::delivery_service::v1::GroupStateEarKey> {
        self.group_state_ear_key.as_ref()
    }
}

struct InvalidGroupStateEarKey(InvalidGroupStateEarKeyLength);

impl From<InvalidGroupStateEarKey> for Status {
    fn from(e: InvalidGroupStateEarKey) -> Self {
        error!(error =% e.0, "Invalid group state ear key");
        Status::invalid_argument("Invalid group state ear key")
    }
}

struct InvalidCiphertext(CiphertextError);

impl From<InvalidCiphertext> for Status {
    fn from(e: InvalidCiphertext) -> Self {
        error!(error =% e.0, "Invalid ciphertext");
        Status::invalid_argument("Invalid ciphertext")
    }
}

struct InvalidQsReference(QsReferenceError);

impl From<InvalidQsReference> for Status {
    fn from(e: InvalidQsReference) -> Self {
        error!(error =% e.0, "Invalid QS reference");
        Status::invalid_argument("Invalid QS reference")
    }
}
