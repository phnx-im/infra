// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::signatures::signable::{self, Signable, SignedStruct},
    messages::client_as::{AsAuthMethod, AsPayload, ClientCredentialAuth},
};
use prost::Message;
use tonic::Status;

use crate::validation::MissingFieldExt;

use super::v1::{Init2FaAuthenticationPayload, Init2FaAuthenticationRequest};

impl Init2FaAuthenticationRequest {
    pub fn into_auth_method(self) -> Result<AsAuthMethod<Init2FaAuthenticationPayload>, Status> {
        let payload = self.payload.ok_or_missing_field(PayloadField)?;
        let client_id = payload
            .client_id
            .as_ref()
            .ok_or_missing_field(ClientIdField)?
            .clone()
            .try_into()?;
        let signature = self.signature.ok_or_missing_field(SignatureField)?.into();
        let auth = ClientCredentialAuth::new(
            client_id,
            payload,
            INIT_2FA_AUTHENTICATION_PAYLOAD_LABEL,
            signature,
        );
        Ok(AsAuthMethod::ClientCredential(auth))
    }
}

impl AsPayload for Init2FaAuthenticationPayload {
    fn is_finish_user_registration_request(&self) -> bool {
        false
    }

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }
}

#[derive(Debug, derive_more::Display)]
#[display(fmt = "payload")]
struct PayloadField;

#[derive(Debug, derive_more::Display)]
#[display(fmt = "signature")]
struct SignatureField;

#[derive(Debug, derive_more::Display)]
#[display(fmt = "client_id")]
struct ClientIdField;

const INIT_2FA_AUTHENTICATION_PAYLOAD_LABEL: &str = "Init2FaAuthenticationPayload";

impl SignedStruct<Init2FaAuthenticationPayload> for Init2FaAuthenticationRequest {
    fn from_payload(payload: Init2FaAuthenticationPayload, signature: signable::Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for Init2FaAuthenticationPayload {
    type SignedOutput = Init2FaAuthenticationRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        INIT_2FA_AUTHENTICATION_PAYLOAD_LABEL
    }
}
