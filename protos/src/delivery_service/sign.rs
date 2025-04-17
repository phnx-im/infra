// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use prost::Message;

use super::v1::{
    CreateGroupPayload, CreateGroupRequest, DeleteGroupPayload, DeleteGroupRequest,
    SendMessagePayload, SendMessageRequest, WelcomeInfoPayload, WelcomeInfoRequest,
};

use phnxtypes::crypto::signatures::signable::{
    self, Signable, Signature, SignedStruct, Verifiable, VerifiedStruct,
};

impl SignedStruct<SendMessagePayload> for SendMessageRequest {
    fn from_payload(payload: SendMessagePayload, signature: signable::Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for SendMessagePayload {
    type SignedOutput = SendMessageRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        "SendMessagePayload"
    }
}

impl VerifiedStruct<SendMessageRequest> for SendMessagePayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: SendMessageRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for SendMessageRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or_else(|| tls_codec::Error::EncodingError("missing payload".to_owned()))?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        "SendMessagePayload"
    }
}

impl SignedStruct<WelcomeInfoPayload> for WelcomeInfoRequest {
    fn from_payload(payload: WelcomeInfoPayload, signature: Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for WelcomeInfoPayload {
    type SignedOutput = WelcomeInfoRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        "WelcomeInfoPayload"
    }
}

impl Verifiable for WelcomeInfoRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or_else(|| tls_codec::Error::EncodingError("missing payload".to_owned()))?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        "WelcomeInfoPayload"
    }
}

impl VerifiedStruct<WelcomeInfoRequest> for WelcomeInfoPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: WelcomeInfoRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl SignedStruct<CreateGroupPayload> for CreateGroupRequest {
    fn from_payload(payload: CreateGroupPayload, signature: Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for CreateGroupPayload {
    type SignedOutput = CreateGroupRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        "CreateGroupPayload"
    }
}

impl VerifiedStruct<CreateGroupRequest> for CreateGroupPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: CreateGroupRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for CreateGroupRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or_else(|| tls_codec::Error::EncodingError("missing payload".to_owned()))?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        "CreateGroupPayload"
    }
}

impl SignedStruct<DeleteGroupPayload> for DeleteGroupRequest {
    fn from_payload(payload: DeleteGroupPayload, signature: Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for DeleteGroupPayload {
    type SignedOutput = DeleteGroupRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        "DeleteGroupPayload"
    }
}

impl VerifiedStruct<DeleteGroupRequest> for DeleteGroupPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: DeleteGroupRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for DeleteGroupRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or_else(|| tls_codec::Error::EncodingError("missing payload".to_owned()))?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        "DeleteGroupPayload"
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}
