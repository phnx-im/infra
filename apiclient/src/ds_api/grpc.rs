use mls_assist::openmls::{group::GroupEpoch, prelude::RatchetTreeIn};
use phnxtypes::crypto::signatures::signable::Signable;
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::ear::keys::GroupStateEarKey,
    identifiers::QualifiedGroupId,
    messages::client_ds_out::{CreateGroupParamsOut, SendMessageParamsOut},
    time::TimeStamp,
};
use protos::delivery_service::v1::{
    CreateGroupRequest, SendMessagePayload, WelcomeInfoPayload,
    delivery_service_client::DeliveryServiceClient,
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
            qgid: Some((&qgid).into()),
            group_state_ear_key: Some(group_state_ear_key.into()),
            ratchet_tree: Some((&params.ratchet_tree).try_into()?),
            encrypted_identity_link_key: Some(params.encrypted_identity_link_key.into()),
            creator_client_reference: Some((&params.creator_client_reference).try_into()?),
            group_info: Some((&params.group_info).try_into()?),
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
            qgid: Some((&qgid).into()),
            group_state_ear_key: Some(group_state_ear_key.into()),
            sender: Some(signing_key.credential().verifying_key().into()),
            epoch: Some(epoch.into()),
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
            .try_into()?)
    }

    pub async fn send_message(
        &self,
        params: SendMessageParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = SendMessagePayload {
            group_state_ear_key: Some(group_state_ear_key.into()),
            message: Some(params.message.try_into()?),
            sender: Some(params.sender.into()),
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
