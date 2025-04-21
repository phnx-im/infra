// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{credentials, crypto::opaque, identifiers, messages, time};
use tls_codec::{DeserializeBytes, Serialize};
use tonic::Status;

use crate::{
    common::convert::QualifiedUserNameError,
    convert::{RefInto, TryFromRef},
    validation::{MissingFieldError, MissingFieldExt},
};

use super::v1::{
    AsClientId, ClientCredential, ClientCredentialCsr, ClientCredentialPayload, ClientPayload,
    ClientVerifyingKey, CredentialFingerprint, ExpirationData, MlsInfraVersion, OpaqueLoginRequest,
    OpaqueLoginResponse, OpaqueRegistrationRequest, OpaqueRegistrationResponse, SignatureScheme,
};

impl From<identifiers::AsClientId> for AsClientId {
    fn from(value: identifiers::AsClientId) -> Self {
        AsClientId {
            user_name: Some(value.user_name().ref_into()),
            client_id: Some(value.client_id().into()),
        }
    }
}

impl TryFrom<AsClientId> for identifiers::AsClientId {
    type Error = AsClientIdError;

    fn try_from(proto: AsClientId) -> Result<Self, Self::Error> {
        Ok(identifiers::AsClientId::new(
            proto
                .user_name
                .ok_or_missing_field("user_name")?
                .try_into()?,
            proto.client_id.ok_or_missing_field("client_id")?.into(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AsClientIdError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    QualifiedUserNameError(#[from] QualifiedUserNameError),
}

impl From<AsClientIdError> for Status {
    fn from(error: AsClientIdError) -> Self {
        Status::invalid_argument(format!("invalid client id: {error}"))
    }
}

impl TryFromRef<'_, opaque::OpaqueLoginRequest> for OpaqueLoginRequest {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &opaque::OpaqueLoginRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, OpaqueLoginRequest> for opaque::OpaqueLoginRequest {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &OpaqueLoginRequest) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl TryFromRef<'_, opaque::OpaqueLoginResponse> for OpaqueLoginResponse {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &opaque::OpaqueLoginResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, OpaqueLoginResponse> for opaque::OpaqueLoginResponse {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &OpaqueLoginResponse) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl From<credentials::ClientCredentialPayload> for ClientPayload {
    fn from(value: credentials::ClientCredentialPayload) -> Self {
        Self {
            csr: Some(value.csr.into()),
            expiration_data: Some(value.expiration_data.into()),
            credential_fingerprint: Some(value.signer_fingerprint.into()),
        }
    }
}

impl TryFrom<ClientPayload> for credentials::ClientCredentialPayload {
    type Error = ClientPayloadError;

    fn try_from(proto: ClientPayload) -> Result<Self, Self::Error> {
        Ok(Self {
            csr: proto.csr.ok_or_missing_field("csr")?.try_into()?,
            expiration_data: proto
                .expiration_data
                .ok_or_missing_field("expiration_data")?
                .try_into()?,
            signer_fingerprint: proto
                .credential_fingerprint
                .ok_or_missing_field("credential_fingerprint")?
                .into(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientPayloadError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    Csr(#[from] ClientCredentialCsrError),
    #[error(transparent)]
    ExpirationData(#[from] ExpirationDataError),
}

impl From<ClientPayloadError> for Status {
    fn from(error: ClientPayloadError) -> Self {
        Status::invalid_argument(format!("invalid client payload: {error}"))
    }
}

impl From<credentials::ClientCredentialCsr> for ClientCredentialCsr {
    fn from(value: credentials::ClientCredentialCsr) -> Self {
        Self {
            msl_version: value.version as u32,
            client_id: Some(value.client_id.into()),
            signature_scheme: value.signature_scheme as i32,
            verifying_key: Some(value.verifying_key.into()),
        }
    }
}

impl TryFrom<ClientCredentialCsr> for credentials::ClientCredentialCsr {
    type Error = ClientCredentialCsrError;

    fn try_from(proto: ClientCredentialCsr) -> Result<Self, Self::Error> {
        let version = match proto.msl_version {
            0 => messages::MlsInfraVersion::Alpha,
            version => return Err(ClientCredentialCsrError::InvalidMlsVersion(version)),
        };
        let signature_scheme = SignatureScheme::try_from(proto.signature_scheme)
            .map_err(|_| UnsupportedSignatureScheme)?
            .try_into()?;

        Ok(Self {
            version,
            client_id: proto
                .client_id
                .ok_or_missing_field("client_id")?
                .try_into()?,
            signature_scheme,
            verifying_key: proto
                .verifying_key
                .ok_or_missing_field("verifying_key")?
                .into(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientCredentialCsrError {
    #[error("invalid MLS version: {0}")]
    InvalidMlsVersion(u32),
    #[error(transparent)]
    Field(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    Signature(#[from] UnsupportedSignatureScheme),
    #[error(transparent)]
    ClientId(#[from] AsClientIdError),
}

#[derive(Debug, thiserror::Error)]
#[error("unsupported MLS version: {0}")]
pub struct UnsupportedMlsVersion(u32);

impl TryFrom<SignatureScheme> for openmls::prelude::SignatureScheme {
    type Error = UnsupportedSignatureScheme;

    fn try_from(value: SignatureScheme) -> Result<Self, Self::Error> {
        use openmls::prelude::SignatureScheme::*;
        match value {
            SignatureScheme::Unspecified => Err(UnsupportedSignatureScheme),
            SignatureScheme::EcdsaSecp256r1Sha256 => Ok(ECDSA_SECP256R1_SHA256),
            SignatureScheme::EcdsaSecp384r1Sha384 => Ok(ECDSA_SECP384R1_SHA384),
            SignatureScheme::EcdsaSecp521r1Sha512 => Ok(ECDSA_SECP521R1_SHA512),
            SignatureScheme::Ed25519 => Ok(ED25519),
            SignatureScheme::Ed448 => Ok(ED448),
        }
    }
}

impl From<openmls::prelude::SignatureScheme> for SignatureScheme {
    fn from(value: openmls::prelude::SignatureScheme) -> Self {
        use openmls::prelude::SignatureScheme::*;
        match value {
            ECDSA_SECP256R1_SHA256 => SignatureScheme::EcdsaSecp256r1Sha256,
            ECDSA_SECP384R1_SHA384 => SignatureScheme::EcdsaSecp384r1Sha384,
            ECDSA_SECP521R1_SHA512 => SignatureScheme::EcdsaSecp521r1Sha512,
            ED25519 => SignatureScheme::Ed25519,
            ED448 => SignatureScheme::Ed448,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unsupported signature scheme")]
pub struct UnsupportedSignatureScheme;

impl From<credentials::keys::ClientVerifyingKey> for ClientVerifyingKey {
    fn from(value: credentials::keys::ClientVerifyingKey) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }
}

impl From<ClientVerifyingKey> for credentials::keys::ClientVerifyingKey {
    fn from(proto: ClientVerifyingKey) -> Self {
        Self::from_bytes(proto.bytes)
    }
}

impl From<messages::MlsInfraVersion> for MlsInfraVersion {
    fn from(value: messages::MlsInfraVersion) -> Self {
        Self {
            version: value as u32,
        }
    }
}

impl TryFrom<MlsInfraVersion> for messages::MlsInfraVersion {
    type Error = UnsupportedMlsVersion;

    fn try_from(value: MlsInfraVersion) -> Result<Self, Self::Error> {
        match value.version {
            1 => Ok(messages::MlsInfraVersion::Alpha),
            _ => Err(UnsupportedMlsVersion(value.version)),
        }
    }
}

impl TryFrom<ExpirationData> for time::ExpirationData {
    type Error = ExpirationDataError;

    fn try_from(value: ExpirationData) -> Result<Self, Self::Error> {
        Ok(Self::from_parts(
            value.not_before.ok_or_missing_field("not_before")?.into(),
            value.not_after.ok_or_missing_field("not_after")?.into(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExpirationDataError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
}

impl From<time::ExpirationData> for ExpirationData {
    fn from(value: time::ExpirationData) -> Self {
        Self {
            not_before: Some(value.not_before().into()),
            not_after: Some(value.not_after().into()),
        }
    }
}

impl From<credentials::CredentialFingerprint> for CredentialFingerprint {
    fn from(value: credentials::CredentialFingerprint) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }
}

impl From<CredentialFingerprint> for credentials::CredentialFingerprint {
    fn from(proto: CredentialFingerprint) -> Self {
        Self::from_bytes(proto.bytes)
    }
}

impl TryFromRef<'_, opaque::OpaqueRegistrationRequest> for OpaqueRegistrationRequest {
    type Error = tls_codec::Error;

    fn try_from_ref(value: &opaque::OpaqueRegistrationRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFromRef<'_, OpaqueRegistrationRequest> for opaque::OpaqueRegistrationRequest {
    type Error = tls_codec::Error;

    fn try_from_ref(proto: &OpaqueRegistrationRequest) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl TryFrom<opaque::OpaqueRegistrationResponse> for OpaqueRegistrationResponse {
    type Error = tls_codec::Error;

    fn try_from(value: opaque::OpaqueRegistrationResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            tls: value.tls_serialize_detached()?,
        })
    }
}

impl TryFrom<OpaqueRegistrationResponse> for opaque::OpaqueRegistrationResponse {
    type Error = tls_codec::Error;

    fn try_from(proto: OpaqueRegistrationResponse) -> Result<Self, Self::Error> {
        DeserializeBytes::tls_deserialize_exact_bytes(&proto.tls)
    }
}

impl TryFrom<ClientCredential> for credentials::VerifiableClientCredential {
    type Error = ClientCredentialError;

    fn try_from(proto: ClientCredential) -> Result<Self, Self::Error> {
        let payload = proto.payload.ok_or_missing_field("payload")?.try_into()?;
        let signature = proto.signature.ok_or_missing_field("signature")?.into();
        Ok(Self::new(payload, signature))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientCredentialError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    Payload(#[from] ClientCredentialPayloadError),
}

impl From<credentials::ClientCredential> for ClientCredential {
    fn from(value: credentials::ClientCredential) -> Self {
        let (payload, signature) = value.into_parts();
        Self {
            payload: Some(payload.into()),
            signature: Some(signature.into()),
        }
    }
}

impl From<credentials::ClientCredentialPayload> for ClientCredentialPayload {
    fn from(value: credentials::ClientCredentialPayload) -> Self {
        Self {
            csr: Some(value.csr.into()),
            expiration_data: Some(value.expiration_data.into()),
            signer_fingerprint: Some(value.signer_fingerprint.into()),
        }
    }
}

impl TryFrom<ClientCredentialPayload> for credentials::ClientCredentialPayload {
    type Error = ClientCredentialPayloadError;

    fn try_from(proto: ClientCredentialPayload) -> Result<Self, Self::Error> {
        let csr = proto.csr.ok_or_missing_field("csr")?.try_into()?;
        let expiration_data = proto.expiration_data.map(TryFrom::try_from).transpose()?;
        let signer_fingerprint = proto
            .signer_fingerprint
            .ok_or_missing_field("credential_fingerprint")?
            .into();
        Ok(credentials::ClientCredentialPayload::new(
            csr,
            expiration_data,
            signer_fingerprint,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientCredentialPayloadError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    Csr(#[from] ClientCredentialCsrError),
    #[error(transparent)]
    ExpirationData(#[from] ExpirationDataError),
}
