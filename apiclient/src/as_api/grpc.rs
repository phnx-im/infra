// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{Init2FaAuthenticationPayload, auth_service_client::AuthServiceClient},
    convert::TryRefInto,
};
use phnxtypes::{
    credentials::keys::ClientSigningKey,
    crypto::{opaque::OpaqueLoginRequest, signatures::signable::Signable},
    identifiers::AsClientId,
    messages::client_as::Init2FactorAuthResponse,
};
use tonic::transport::Channel;

use super::AsRequestError;

#[derive(Clone)]
pub(crate) struct AsGrpcClient {
    client: AuthServiceClient<Channel>,
}

impl AsGrpcClient {
    pub(crate) fn new(client: AuthServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub(crate) async fn initiate_2fa_auth(
        &self,
        client_id: AsClientId,
        opaque_ke1: OpaqueLoginRequest,
        signing_key: &ClientSigningKey,
    ) -> Result<Init2FactorAuthResponse, AsRequestError> {
        let payload = Init2FaAuthenticationPayload {
            client_id: Some(client_id.into()),
            opaque_ke1: Some(opaque_ke1.try_ref_into()?),
        };
        let request = payload.sign(signing_key)?;
        let response = self
            .client
            .clone()
            .init2_fa_authentication(request)
            .await?
            .into_inner();
        let opaque_ke2 = response
            .opaque_ke2
            .ok_or(AsRequestError::UnexpectedResponse)?
            .try_ref_into()
            .map_err(|_| AsRequestError::UnexpectedResponse)?;
        Ok(Init2FactorAuthResponse { opaque_ke2 })
    }
}
