use phnxtypes::crypto::signatures::signable::Signable;
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::{ear::keys::GroupStateEarKey, signatures::traits::SigningKeyBehaviour},
    identifiers::QualifiedGroupId,
    messages::client_ds_out::{CreateGroupParamsOut, SendMessageParamsOut},
    time::TimeStamp,
};
use prost::Message;
use protos::{
    SIGNATURE_METADATA_KEY,
    delivery_service::v1::{
        CreateGroupRequest, SendMessagePayload, delivery_service_client::DeliveryServiceClient,
    },
};
use tonic::{metadata::MetadataValue, transport::Channel};
use tracing::info;

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
        payload: CreateGroupParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        info!("ds_create_group via grpc");

        let qgid: QualifiedGroupId = (&payload.group_id).try_into()?;
        let request = CreateGroupRequest {
            qgid: Some((&qgid).into()),
            group_state_ear_key: Some(group_state_ear_key.into()),
            ratchet_tree: Some((&payload.ratchet_tree).try_into()?),
            encrypted_identity_link_key: Some(payload.encrypted_identity_link_key.into()),
            creator_client_reference: Some((&payload.creator_client_reference).try_into()?),
            group_info: Some((&payload.group_info).try_into()?),
        };

        let signature = signing_key.sign(&request.encode_to_vec())?;

        let mut request = tonic::Request::new(request);
        request.metadata_mut().insert_bin(
            SIGNATURE_METADATA_KEY,
            MetadataValue::from_bytes(&signature.into_bytes()),
        );

        self.client.clone().create_group(request).await?;

        Ok(())
    }

    pub async fn send_message(
        &self,
        params: SendMessageParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        info!("ds_send_message via grpc");

        let tbs = SendMessagePayload {
            group_state_ear_key: Some(group_state_ear_key.into()),
            message: Some(params.message.try_into()?),
            sender: Some(params.sender.into()),
        };

        let request = tbs.sign(signing_key)?;
        let response = self.client.clone().send_message(request).await?;

        Ok(response
            .into_inner()
            .fanout_timestamp
            .ok_or(DsRequestError::UnexpectedResponse)?
            .into())
    }
}
