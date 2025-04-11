use std::borrow::Cow;

use prost::Message;

use super::v1::{SendMessagePayload, SendMessageRequest};

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

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

impl VerifiedStruct<SendMessageRequest> for SendMessagePayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: SendMessageRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}
