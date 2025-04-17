// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::GroupId;
use phnxprotos::{
    convert::{RefInto, TryRefInto},
    delivery_service::v1::{
        CreateGroupPayload, RequestGroupIdRequest, SendMessagePayload,
        delivery_service_client::DeliveryServiceClient,
    },
    validation::MissingFieldExt,
};
use phnxtypes::{
    credentials::keys::PseudonymousCredentialSigningKey,
    crypto::{ear::keys::GroupStateEarKey, signatures::signable::Signable},
    identifiers::QualifiedGroupId,
    messages::client_ds_out::{CreateGroupParamsOut, SendMessageParamsOut},
    time::TimeStamp,
};
use tonic::transport::Channel;
use tracing::error;

use super::DsRequestError;

#[derive(Clone)]
pub(crate) struct DsGrpcClient {
    client: DeliveryServiceClient<Channel>,
}

impl DsGrpcClient {
    pub(crate) fn new(client: DeliveryServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(crate) async fn request_group_id(&self) -> Result<GroupId, DsRequestError> {
        let response = self
            .client
            .clone()
            .request_group_id(RequestGroupIdRequest {})
            .await?
            .into_inner();
        let qgid: QualifiedGroupId = response
            .group_id
            .ok_or_missing_field("group_id")
            .map_err(|error| {
                error!(%error, "unexpected response");
                DsRequestError::UnexpectedResponse
            })?
            .try_ref_into()
            .map_err(|error| {
                error!(%error, "unexpected response");
                DsRequestError::UnexpectedResponse
            })?;
        Ok(qgid.into())
    }

    pub(crate) async fn send_message(
        &self,
        params: SendMessageParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<TimeStamp, DsRequestError> {
        let payload = SendMessagePayload {
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            message: Some(params.message.try_ref_into()?),
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

    pub(crate) async fn create_group(
        &self,
        payload: CreateGroupParamsOut,
        signing_key: &PseudonymousCredentialSigningKey,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(), DsRequestError> {
        let qgid: QualifiedGroupId = payload.group_id.try_into()?;
        let payload = CreateGroupPayload {
            qgid: Some(qgid.ref_into()),
            group_state_ear_key: Some(group_state_ear_key.ref_into()),
            ratchet_tree: Some(payload.ratchet_tree.try_ref_into()?),
            encrypted_identity_link_key: Some(payload.encrypted_identity_link_key.into()),
            encrypted_user_profile_key: Some(payload.encrypted_user_profile_key.into()),
            creator_client_reference: Some(payload.creator_client_reference.try_ref_into()?),
            group_info: Some(payload.group_info.try_ref_into()?),
        };
        let request = payload.sign(signing_key)?;
        self.client.clone().create_group(request).await?;
        Ok(())
    }
}
