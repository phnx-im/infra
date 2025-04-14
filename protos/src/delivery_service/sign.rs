use std::borrow::Cow;

use prost::Message;

use super::v1::{SendMessagePayload, SendMessageRequest, WelcomeInfoPayload, WelcomeInfoRequest};

use phnxtypes::crypto::signatures::signable::{
    Signable, Signature, SignedStruct, Verifiable, VerifiedStruct,
};

impl SignedStruct<SendMessagePayload> for SendMessageRequest {
    fn from_payload(
        payload: SendMessagePayload,
        signature: phnxtypes::crypto::signatures::signable::Signature,
    ) -> Self {
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
            .ok_or_else(|| tls_codec::Error::EncodingError("missing payload".to_string()))?
            .encode_to_vec())
    }

    fn signature(&self) -> Cow<Signature> {
        Cow::Owned(self.signature.clone().unwrap_or_default().into())
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
            .ok_or_else(|| tls_codec::Error::EncodingError("missing payload".to_string()))?
            .encode_to_vec())
    }

    fn signature(&self) -> Cow<Signature> {
        Cow::Owned(self.signature.clone().unwrap_or_default().into())
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

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}
