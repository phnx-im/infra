#![allow(unused_variables)]

use phnxtypes::{
    crypto::{
        ear::{
            Ciphertext,
            keys::{EncryptedIdentityLinkKey, GroupStateEarKey},
        },
        signatures::{keys::LeafVerifyingKey, signable::Signature, traits::VerifyingKeyBehaviour},
    },
    identifiers::{Fqdn, QualifiedGroupId},
    messages::client_ds::DsSender,
};
use prost::Message;
use protos::delivery_service::v1::{
    ConnectionGroupInfoRequest, ConnectionGroupInfoResponse, CreateGroupRequest,
    CreateGroupResponse, DeleteGroupRequest, DeleteGroupResponse, ExternalCommitInfoRequest,
    ExternalCommitInfoResponse, GroupOperationRequest, GroupOperationResponse,
    JoinConnectionGroupRequest, JoinConnectionGroupResponse, RequestGroupIdRequest,
    RequestGroupIdResponse, ResyncRequest, ResyncResponse, SelfRemoveRequest, SelfRemoveResponse,
    SendMessageRequest, SendMessageResponse, UpdateQsClientReferenceRequest,
    UpdateQsClientReferenceResponse, UpdateRequest, UpdateResponse, WelcomeInfoRequest,
    WelcomeInfoResponse,
};
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use super::{
    Ds,
    group_state::{DsGroupState, StorableDsGroupData},
};

const SIGNATURE_METADATA_KEY: &str = "signature-bin";

#[async_trait]
impl protos::delivery_service::v1::delivery_service_server::DeliveryService for Ds {
    async fn request_group_id(
        &self,
        _request: Request<RequestGroupIdRequest>,
    ) -> Result<Response<RequestGroupIdResponse>, Status> {
        let group_id = self.request_group_id().await;
        Ok(Response::new(RequestGroupIdResponse {
            group_id: Some(group_id.into()),
        }))
    }

    async fn create_group(
        &self,
        request: Request<CreateGroupRequest>,
    ) -> Result<Response<CreateGroupResponse>, Status> {
        let message = request.into_inner();

        let qgid = get_qgid(message.qgid, &self.own_domain)?;
        let ear_key = get_group_state_ear_key(message.group_state_ear_key)?;

        let leaf_node = message
            .ratchet_tree
            .ok_or_else(|| Status::invalid_argument("Missing ratchet tree"))?
            .try_into()
            .map_err(|error| {
                error!(%error, "Invalid ratchet tree");
                Status::invalid_argument("Invalid ratchet tree")
            })?;

        let ciphertext: Ciphertext = message
            .encrypted_identity_link_key
            .ok_or_else(|| Status::invalid_argument("Missing encrypted identity link key"))?
            .ciphertext
            .ok_or_else(|| {
                Status::invalid_argument("Missing encrypted identity link key ciphertext")
            })?
            .try_into()
            .map_err(|error| {
                error!(%error, "Invalid encrypted identity link key");
                Status::invalid_argument("Invalid encrypted identity link key")
            })?;
        let encrypted_identity_link_key = EncryptedIdentityLinkKey::from(ciphertext);

        let creator_qs_reference = message
            .creator_client_reference
            .ok_or_else(|| Status::invalid_argument("Missing creator qs reference"))?
            .try_into()
            .map_err(|error| {
                error!(%error, "Invalid creator qs reference");
                Status::invalid_argument("Invalid creator qs reference")
            })?;

        let group_info = message
            .group_info
            .ok_or_else(|| Status::invalid_argument("Missing group info"))?
            .try_into()
            .map_err(|error| {
                error!(%error, "Invalid group info");
                Status::invalid_argument("Invalid group info")
            })?;

        let (reserved_group_id, group_state) = self
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
        StorableDsGroupData::new_and_store(self.pool(), reserved_group_id, encrypted_group_state)
            .await?;

        Ok(CreateGroupResponse {}.into())
    }

    async fn welcome_info(
        &self,
        request: Request<WelcomeInfoRequest>,
    ) -> Result<Response<WelcomeInfoResponse>, Status> {
        let signature = request
            .metadata()
            .get_bin(SIGNATURE_METADATA_KEY)
            .ok_or(Status::invalid_argument("Missing signature"))?
            .as_encoded_bytes();
        let signature = Signature::from_bytes(signature.to_vec());

        let mut message = request.into_inner();
        message.verify_signature(&signature)?;

        let qgid = get_qgid(message.qgid.take(), &self.own_domain)?;
        let ear_key = get_group_state_ear_key(message.group_state_ear_key.take())?;

        let mut group_data = StorableDsGroupData::load(&self.db_pool, &qgid)
            .await?
            .ok_or(Status::not_found("Group not found"))?;
        if group_data.has_expired() {
            StorableDsGroupData::delete(&self.db_pool, &qgid).await?;
            return Err(Status::not_found("Group not found"));
        }
        let mut group_state = DsGroupState::decrypt(&group_data.encrypted_group_state, &ear_key)?;

        let epoch = message
            .epoch
            .ok_or_else(|| Status::invalid_argument("Missing epoch"))?
            .into();
        let joiner = message
            .sender
            .ok_or_else(|| Status::invalid_argument("Missing sender"))?
            .bytes
            .into();
        let ratchet_tree = group_state
            .group_mut()
            .past_group_state(&epoch, &joiner)
            .ok_or_else(|| Status::not_found("No welcome info found"))?
            .clone();

        group_data.encrypted_group_state = group_state.encrypt(&ear_key)?;
        group_data.update(&self.db_pool).await?;

        let response = WelcomeInfoResponse {
            ratchet_tree: Some(ratchet_tree.try_into().map_err(|error| {
                error!(%error, "Failed to serialize ratchet tree");
                Status::internal("Failed to serialize ratchet tree")
            })?),
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
        todo!()
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

fn get_qgid(
    qgid: Option<protos::common::v1::QualifiedGroupId>,
    own_domain: &Fqdn,
) -> Result<QualifiedGroupId, Status> {
    let qgid: QualifiedGroupId = qgid
        .ok_or_else(|| Status::invalid_argument("Missing group id"))?
        .try_into()
        .map_err(|error| {
            error!(%error, "Invalid group id");
            Status::invalid_argument("Invalid group id")
        })?;

    if qgid.owning_domain() != own_domain {
        error!(
            domain =% qgid.owning_domain(),
            "Group id domain does not match own domain"
        );
        return Err(Status::invalid_argument(
            "Group id domain does not match own domain",
        ));
    }

    Ok(qgid)
}

fn get_group_state_ear_key(
    key: Option<protos::delivery_service::v1::GroupStateEarKey>,
) -> Result<GroupStateEarKey, Status> {
    key.ok_or_else(|| Status::invalid_argument("Missing group state ear key"))?
        .try_into()
        .map_err(|error| {
            error!(%error, "Invalid group state ear key");
            Status::invalid_argument("Invalid group state ear key")
        })
}

trait MessageExt: Message + Sized {
    fn ds_sender(&self) -> Result<DsSender, Status>;

    fn verify_signature(&self, signature: &Signature) -> Result<(), Status> {
        match self.ds_sender()? {
            DsSender::LeafSignatureKey(verifying_key) => {
                let payload = self.encode_to_vec();
                let public_key = LeafVerifyingKey::from(&verifying_key);
                public_key.verify(&payload, signature).map_err(|error| {
                    error!(%error, "Invalid signature");
                    Status::invalid_argument("Invalid signature")
                })?;
            }
            _ => todo!(),
        }
        Ok(())
    }
}

impl MessageExt for protos::delivery_service::v1::WelcomeInfoRequest {
    fn ds_sender(&self) -> Result<DsSender, Status> {
        let signature = self
            .sender
            .clone()
            .ok_or_else(|| Status::invalid_argument("Missing sender"))?;
        Ok(DsSender::LeafSignatureKey(signature.bytes.into()))
    }
}
