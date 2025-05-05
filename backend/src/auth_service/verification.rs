// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::signatures::signable::Verifiable,
    errors::auth_service::AsVerificationError,
    messages::{ApiVersion, client_as::AsAuthMethod, client_as_out::ClientToAsMessageIn},
};
use tls_codec::TlsDeserializeBytes;

use super::{AuthService, TlsSize, VerifiedAsRequestParams, client_record::ClientRecord};

/// Wrapper struct around a message from a client to the AS. It does not
/// implement the [`Verifiable`] trait, but instead is verified depending on the
/// verification method of the individual payload.
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct VerifiableClientToAsMessage(ClientToAsMessageIn);

impl AuthService {
    pub(crate) async fn verify(
        &self,
        message: VerifiableClientToAsMessage,
    ) -> Result<(VerifiedAsRequestParams, ApiVersion), AsVerificationError> {
        let versioned_params = message.0.into_body();
        let (params, version) = versioned_params.into_unversioned()?;
        let params = match params.into_auth_method() {
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
        };
        Ok((params, version))
    }
}
