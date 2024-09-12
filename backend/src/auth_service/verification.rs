// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::signatures::signable::Verifiable,
    errors::auth_service::AsVerificationError,
    messages::{client_as::AsAuthMethod, client_as_out::ClientToAsMessageIn},
};
use tls_codec::TlsDeserializeBytes;

use super::{client_record::ClientRecord, AuthService, TlsSize, VerifiedAsRequestParams};

/// Wrapper struct around a message from a client to the AS. It does not
/// implement the [`Verifiable`] trait, but instead is verified depending on the
/// verification method of the individual payload.
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct VerifiableClientToAsMessage(ClientToAsMessageIn);

impl VerifiableClientToAsMessage {
    fn into_auth_method(self) -> AsAuthMethod {
        self.0.auth_method()
    }
}

impl AuthService {
    pub(crate) async fn verify(
        &self,
        message: VerifiableClientToAsMessage,
    ) -> Result<VerifiedAsRequestParams, AsVerificationError> {
        let parameters = match message.into_auth_method() {
            // No authentication at all. We just return the parameters without
            // verification.
            AsAuthMethod::None(params) => params,
            // Authentication via client credential. We load the client
            // credential from the client's record and use it to verify the
            // request.
            AsAuthMethod::ClientCredential(cca) => {
                // Depending on the request type, we either load the client
                // credential from the persistend storage, or the ephemeral
                // storage.
                if cca.is_finish_user_registration_request() {
                    let client_credentials = self.ephemeral_client_credentials.lock().await;
                    let client_credential = client_credentials
                        .get(cca.client_id())
                        .ok_or(AsVerificationError::UnknownClient)?;
                    cca.verify(client_credential.verifying_key())
                        .map_err(|_| AsVerificationError::AuthenticationFailed)?
                } else {
                    let client_record = ClientRecord::load(&self.db_pool, cca.client_id())
                        .await
                        .map_err(|e| {
                            tracing::error!("Error loading client record: {:?}", e);
                            AsVerificationError::UnknownClient
                        })?
                        .ok_or(AsVerificationError::UnknownClient)?;
                    cca.verify(client_record.credential.verifying_key())
                        .map_err(|_| AsVerificationError::AuthenticationFailed)?
                }
            }
            // 2-Factor authentication using a signature by the client
            // credential, as well as an OPAQUE login flow. This requires that
            // the client has first called the endpoint to initiate the OPAQUE
            // login flow.
            // We load the pending OPAQUE login state from the ephemeral
            // database and complete the OPAQUE flow. If that is successful, we
            // verify the signature (which spans the OPAQUE data sent by the
            // client).
            // After successful verification, we delete the entry from the
            // ephemeral DB.
            // TODO: We currently store the credential of the client to be added
            // along with the OPAQUE entry. This is not great, since we can't
            // really return it from here. For now, we just load it again from
            // the processing function.
            AsAuthMethod::Client2Fa(auth_info) => {
                // We authenticate opaque first.
                let client_id = auth_info.client_credential_auth.client_id().clone();
                let mut client_login_states = self.ephemeral_client_logins.lock().await;
                let opaque_state = client_login_states
                    .remove(&client_id)
                    .ok_or(AsVerificationError::UnknownClient)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(auth_info.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::warn!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                let client_record = ClientRecord::load(&self.db_pool, &client_id)
                    .await
                    .map_err(|e| {
                        tracing::error!("Error loading client record: {:?}", e);
                        AsVerificationError::UnknownClient
                    })?
                    .ok_or(AsVerificationError::UnknownClient)?;
                let verified_params = auth_info
                    .client_credential_auth
                    .verify(client_record.credential.verifying_key())
                    .map_err(|_| AsVerificationError::AuthenticationFailed)?;
                verified_params
            }
            // Authentication using only the user's password via an OPAQUE login flow.
            AsAuthMethod::User(user_auth) => {
                let mut user_login_states = self.ephemeral_user_logins.lock().await;
                let opaque_state = user_login_states
                    .remove(&user_auth.user_name)
                    .ok_or(AsVerificationError::UnknownUser)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(user_auth.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::error!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;
                *user_auth.payload
            }
        };
        Ok(parameters)
    }
}
