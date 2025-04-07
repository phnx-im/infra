#![allow(unused_variables)]

use phnxtypes::{
    crypto::ear::{
        Ciphertext,
        keys::{EncryptedIdentityLinkKey, GroupStateEarKey, GroupStateEarKeySecret},
    },
    identifiers::{Fqdn, QualifiedGroupId},
    messages::client_ds::CreateGroupParams,
};
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
use uuid::Uuid;

use super::{Ds, group_state::StorableDsGroupData};

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
        let Some((qgid, ear_key)) = message.group_id_and_ear_key() else {
            return Err(Status::invalid_argument("Invalid group id or ear key"));
        };

        if qgid.owning_domain() != self.own_domain() {
            error!(
                domain =% qgid.owning_domain(),
                "Group id does not belong to own domain"
            );
            return Err(Status::invalid_argument(
                "Group id does not belong to own domain",
            ));
        }

        let group_id = message
            .group_id
            .ok_or_else(|| Status::invalid_argument("Missing group id"))?
            .into();

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

        let params = CreateGroupParams {
            group_id,
            leaf_node,
            encrypted_identity_link_key,
            creator_qs_reference,
            group_info,
        };

        let (reserved_group_id, group_state) =
            self.create_group(&qgid, &params).await.map_err(|error| {
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
        todo!()
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

trait RequestMessageExt {
    fn group_id_and_ear_key(&self) -> Option<(QualifiedGroupId, GroupStateEarKey)>;
}

impl RequestMessageExt for CreateGroupRequest {
    fn group_id_and_ear_key(&self) -> Option<(QualifiedGroupId, GroupStateEarKey)> {
        let qgid = self.qgid.clone()?;
        let group_uuid: Uuid = (*qgid.group_uuid.as_ref()?).into();
        let fqdn: Fqdn = qgid.domain.as_ref()?.value.parse().ok()?;
        let qgid = QualifiedGroupId::new(group_uuid, fqdn);

        let bytes: [u8; 32] = self
            .group_state_ear_key
            .as_ref()?
            .key
            .as_slice()
            .try_into()
            .unwrap();
        let key = GroupStateEarKeySecret::from(bytes);
        let key = GroupStateEarKey::from(key);

        Some((qgid, key))
    }
}
