use phnxtypes::crypto::signatures::signable::{
    self, Signable, SignedStruct, Verifiable, VerifiedStruct,
};
use prost::Message;

use super::v1::{
    DeleteUserPayload, DeleteUserRequest, DequeueMessagesPayload, DequeueMessagesRequest,
    PublishConnectionPackagesPayload, PublishConnectionPackagesRequest,
};

const DELETE_USER_PAYLOAD_LABEL: &str = "DeleteUserPayload";

impl SignedStruct<DeleteUserPayload> for DeleteUserRequest {
    fn from_payload(payload: DeleteUserPayload, signature: signable::Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for DeleteUserPayload {
    type SignedOutput = DeleteUserRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        DELETE_USER_PAYLOAD_LABEL
    }
}

impl VerifiedStruct<DeleteUserRequest> for DeleteUserPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: DeleteUserRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for DeleteUserRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or(MissingPayloadError)?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        DELETE_USER_PAYLOAD_LABEL
    }
}

const PUBLISH_CONNECTION_PACKAGES_PAYLOAD_LABEL: &str = "PublishConnectionPackagesPayload";

impl SignedStruct<PublishConnectionPackagesPayload> for PublishConnectionPackagesRequest {
    fn from_payload(
        payload: PublishConnectionPackagesPayload,
        signature: signable::Signature,
    ) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for PublishConnectionPackagesPayload {
    type SignedOutput = PublishConnectionPackagesRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        PUBLISH_CONNECTION_PACKAGES_PAYLOAD_LABEL
    }
}

impl VerifiedStruct<PublishConnectionPackagesRequest> for PublishConnectionPackagesPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(
        verifiable: PublishConnectionPackagesRequest,
        _seal: Self::SealingType,
    ) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for PublishConnectionPackagesRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or(MissingPayloadError)?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        PUBLISH_CONNECTION_PACKAGES_PAYLOAD_LABEL
    }
}

struct MissingPayloadError;

impl From<MissingPayloadError> for tls_codec::Error {
    fn from(_: MissingPayloadError) -> Self {
        tls_codec::Error::EncodingError("missing payload".to_owned())
    }
}

const DEQUEUE_MESSAGES_PAYLOAD_LABEL: &str = "DequeueMessagesPayload";

impl SignedStruct<DequeueMessagesPayload> for DequeueMessagesRequest {
    fn from_payload(payload: DequeueMessagesPayload, signature: signable::Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for DequeueMessagesPayload {
    type SignedOutput = DequeueMessagesRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        DEQUEUE_MESSAGES_PAYLOAD_LABEL
    }
}

impl VerifiedStruct<DequeueMessagesRequest> for DequeueMessagesPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: DequeueMessagesRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for DequeueMessagesRequest {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self
            .payload
            .as_ref()
            .ok_or(MissingPayloadError)?
            .encode_to_vec())
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        self.signature
            .as_ref()
            .map(|s| s.value.as_slice())
            .unwrap_or_default()
    }

    fn label(&self) -> &str {
        DEQUEUE_MESSAGES_PAYLOAD_LABEL
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}
