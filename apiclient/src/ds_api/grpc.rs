use mls_assist::openmls::{group::GroupEpoch, prelude::RatchetTreeIn};
use phnxtypes::crypto::signatures::signable::Signable;
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::ear::keys::GroupStateEarKey,
    identifiers::QualifiedGroupId,
    messages::client_ds_out::{CreateGroupParamsOut, SendMessageParamsOut},
    time::TimeStamp,
};
use protos::{
    IntoProto, ToProto, TryToProto,
    delivery_service::v1::{
        CreateGroupRequest, SendMessagePayload, WelcomeInfoPayload,
        delivery_service_client::DeliveryServiceClient,
    },
};
use tonic::transport::Channel;

use super::DsRequestError;

#[derive(Clone)]
pub(crate) struct GrpcDsClient {
    client: DeliveryServiceClient<Channel>,
}

impl GrpcDsClient {
    pub(crate) fn new(client: DeliveryServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(crate) async fn create_group(
        &self,
        params: CreateGroupParamsOut,
        _signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let qgid: QualifiedGroupId = (&params.group_id).try_into()?;
        let request = CreateGroupRequest {
            qgid: Some(qgid.to_proto()),
            group_state_ear_key: Some(group_state_ear_key.to_proto()),
            ratchet_tree: Some(params.ratchet_tree.try_to_proto()?),
            encrypted_identity_link_key: Some(params.encrypted_identity_link_key.into_proto()),
            creator_client_reference: Some(params.creator_client_reference.try_to_proto()?),
            group_info: Some(params.group_info.try_to_proto()?),
        };
        self.client.clone().create_group(request).await?;
        Ok(())
    }

    pub(crate) async fn welcome_info(
        &self,
        qgid: QualifiedGroupId,
        epoch: GroupEpoch,
        group_state_ear_key: &GroupStateEarKey,
        signing_key: &PseudonymousCredentialSigningKey,
    ) -> Result<RatchetTreeIn, DsRequestError> {
        let payload = WelcomeInfoPayload {
            qgid: Some(qgid.to_proto()),
            group_state_ear_key: Some(group_state_ear_key.to_proto()),
            sender: Some(signing_key.credential().verifying_key().to_proto()),
            epoch: Some(epoch.into_proto()),
        };

        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .welcome_info(request)
            .await?
            .into_inner();

        Ok(response
            .ratchet_tree
            .ok_or(DsRequestError::UnexpectedResponse)?
            .try_to_typed()?)
    }

    pub async fn send_message(
        &self,
        params: SendMessageParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = SendMessagePayload {
            group_state_ear_key: Some(group_state_ear_key.to_proto()),
            message: Some(params.message.try_to_proto()?),
            sender: Some(params.sender.into_proto()),
        };

        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .send_message(request)
            .await?
            .into_inner();

        Ok(response
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }
}
