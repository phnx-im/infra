// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::signatures::signable::Verifiable,
    messages::{client_as::AsAuthMethod, client_as_out::ClientToAsMessageIn},
};
use tls_codec::TlsDeserializeBytes;

use super::{errors::AsVerificationError, *};

/// Wrapper struct around a message from a client to the AS. It does not
/// implement the [`Verifiable`] trait, but instead is verified depending on the
/// verification method of the individual payload.
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct VerifiableClientToAsMessage {
    message: ClientToAsMessageIn,
}

impl VerifiableClientToAsMessage {
    /// Verify/authenticate the message. The authentication method depends on
    /// the request type and is specified for each request in `auth_method`.
    pub(crate) async fn verify<Asp: AsStorageProvider, Eph: AsEphemeralStorageProvider>(
        self,
        as_storage_provider: &Asp,
        ephemeral_storage_provider: &Eph,
    ) -> Result<VerifiedAsRequestParams, AsVerificationError> {
        let parameters = match self.message.auth_method() {
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
                    let client_credential = ephemeral_storage_provider
                        .load_credential(cca.client_id())
                        .await
                        .ok_or(AsVerificationError::UnknownClient)?;
                    cca.verify(client_credential.verifying_key())
                        .map_err(|_| AsVerificationError::AuthenticationFailed)?
                } else {
                    let client_record = as_storage_provider
                        .load_client(cca.client_id())
                        .await
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
                let (_client_credential, opaque_state) = ephemeral_storage_provider
                    .load_client_login_state(&client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownClient)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(auth_info.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::warn!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                let client_record = as_storage_provider
                    .load_client(&client_id)
                    .await
                    .ok_or(AsVerificationError::UnknownClient)?;
                let verified_params = auth_info
                    .client_credential_auth
                    .verify(client_record.credential.verifying_key())
                    .map_err(|_| AsVerificationError::AuthenticationFailed)?;
                ephemeral_storage_provider
                    .delete_client_login_state(&client_id)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?;
                verified_params
            }
            // Authentication using only the user's password via an OPAQUE login flow.
            AsAuthMethod::User(user_auth) => {
                let opaque_state = ephemeral_storage_provider
                    .load_user_login_state(&user_auth.user_name)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?
                    .ok_or(AsVerificationError::UnknownUser)?;
                // Finish the OPAQUE handshake
                opaque_state
                    .finish(user_auth.opaque_finish.client_message)
                    .map_err(|e| {
                        tracing::error!("Error during OPAQUE login handshake: {e}");
                        AsVerificationError::AuthenticationFailed
                    })?;

                ephemeral_storage_provider
                    .delete_user_login_state(&user_auth.user_name)
                    .await
                    .map_err(|_| AsVerificationError::StorageError)?;
                *user_auth.payload
            }
        };
        Ok(parameters)
    }
}
