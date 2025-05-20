// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::crypto::signatures::signable::{
    self, Signable, SignedStruct, Verifiable, VerifiedStruct,
};
use prost::Message;

use super::v1::{
    DeleteUserPayload, DeleteUserRequest, InitListenPayload, InitListenRequest,
    MergeUserProfilePayload, MergeUserProfileRequest, PublishConnectionPackagesPayload,
    PublishConnectionPackagesRequest, StageUserProfilePayload, StageUserProfileRequest,
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

const STAGE_USER_PROFILE_PAYLOAD_LABEL: &str = "StageUserProfilePayload";

impl SignedStruct<StageUserProfilePayload> for StageUserProfileRequest {
    fn from_payload(payload: StageUserProfilePayload, signature: signable::Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for StageUserProfilePayload {
    type SignedOutput = StageUserProfileRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        STAGE_USER_PROFILE_PAYLOAD_LABEL
    }
}

impl VerifiedStruct<StageUserProfileRequest> for StageUserProfilePayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: StageUserProfileRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for StageUserProfileRequest {
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
        STAGE_USER_PROFILE_PAYLOAD_LABEL
    }
}

const MERGE_USER_PROFILE_PAYLOAD_LABEL: &str = "MergeUserProfilePayload";

impl SignedStruct<MergeUserProfilePayload> for MergeUserProfileRequest {
    fn from_payload(payload: MergeUserProfilePayload, signature: signable::Signature) -> Self {
        Self {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for MergeUserProfilePayload {
    type SignedOutput = MergeUserProfileRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        MERGE_USER_PROFILE_PAYLOAD_LABEL
    }
}

impl VerifiedStruct<MergeUserProfileRequest> for MergeUserProfilePayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: MergeUserProfileRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for MergeUserProfileRequest {
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
        MERGE_USER_PROFILE_PAYLOAD_LABEL
    }
}

const INIT_LISTEN_REQUEST_LABEL: &str = "InitListenRequest";

impl SignedStruct<InitListenPayload> for InitListenRequest {
    fn from_payload(payload: InitListenPayload, signature: signable::Signature) -> Self {
        InitListenRequest {
            payload: Some(payload),
            signature: Some(signature.into()),
        }
    }
}

impl Signable for InitListenPayload {
    type SignedOutput = InitListenRequest;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.encode_to_vec())
    }

    fn label(&self) -> &str {
        INIT_LISTEN_REQUEST_LABEL
    }
}

impl VerifiedStruct<InitListenRequest> for InitListenPayload {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: InitListenRequest, _seal: Self::SealingType) -> Self {
        verifiable.payload.unwrap()
    }
}

impl Verifiable for InitListenRequest {
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
        INIT_LISTEN_REQUEST_LABEL
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}
