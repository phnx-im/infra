// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxprotos::{
    auth_service::v1::{
        FinishUserRegistrationPayload, Init2FaAuthenticationPayload, InitUserRegistrationRequest,
        auth_service_client::AuthServiceClient,
    },
    convert::TryRefInto,
};
use phnxtypes::{
    credentials::{ClientCredentialPayload, keys::ClientSigningKey},
    crypto::{
        RatchetEncryptionKey,
        kdf::keys::RatchetSecret,
        opaque::{OpaqueLoginRequest, OpaqueRegistrationRecord, OpaqueRegistrationRequest},
        signatures::signable::Signable,
    },
    identifiers::AsClientId,
    messages::{
        client_as::{ConnectionPackage, Init2FactorAuthResponse},
        client_as_out::{EncryptedUserProfile, InitUserRegistrationResponseIn},
    },
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

    pub(crate) async fn initiate_create_user(
        &self,
        client_payload: ClientCredentialPayload,
        opaque_registration_request: OpaqueRegistrationRequest,
    ) -> Result<InitUserRegistrationResponseIn, AsRequestError> {
        let request = InitUserRegistrationRequest {
            client_payload: Some(client_payload.into()),
            opaque_registration_request: Some(opaque_registration_request.try_ref_into()?),
        };
        let response = self
            .client
            .clone()
            .init_user_registration(request)
            .await?
            .into_inner();
        Ok(InitUserRegistrationResponseIn {
            client_credential: response
                .client_credential
                .ok_or(AsRequestError::UnexpectedResponse)?
                .try_into()
                .map_err(|_| AsRequestError::UnexpectedResponse)?,
            opaque_registration_response: response
                .opaque_registration_response
                .ok_or(AsRequestError::UnexpectedResponse)?
                .try_into()?,
        })
    }

    pub(crate) async fn finish_user_registration(
        &self,
        queue_encryption_key: RatchetEncryptionKey,
        initial_ratchet_secret: RatchetSecret,
        connection_packages: Vec<ConnectionPackage>,
        opaque_registration_record: OpaqueRegistrationRecord,
        signing_key: &ClientSigningKey,
        encrypted_user_profile: EncryptedUserProfile,
    ) -> Result<(), AsRequestError> {
        let payload = FinishUserRegistrationPayload {
            client_id: Some(signing_key.credential().identity().clone().into()),
            queue_encryption_key: Some(queue_encryption_key.into()),
            initial_ratchet_secret: Some(initial_ratchet_secret.into()),
            connection_packages: connection_packages.into_iter().map(Into::into).collect(),
            opaque_registration_record: Some(opaque_registration_record.try_into()?),
            encrypted_user_profile: Some(encrypted_user_profile.into()),
        };
        let request = payload.sign(signing_key)?;
        self.client
            .clone()
            .finish_user_registration(request)
            .await?
            .into_inner();
        Ok(())
    }
}
