// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{credentials, crypto, identifiers, messages, time};
use tonic::Status;

use crate::{
    common::convert::{InvalidIndexedCiphertext, QualifiedUserNameError},
    validation::{MissingFieldError, MissingFieldExt},
};

use super::v1::{
    AsClientId, ClientCredential, ClientCredentialCsr, ClientCredentialPayload, ClientVerifyingKey,
    ConnectionEncryptionKey, ConnectionPackage, ConnectionPackagePayload, CredentialFingerprint,
    EncryptedUserProfile, ExpirationData, MlsInfraVersion, SignatureScheme,
};

impl From<identifiers::AsClientId> for AsClientId {
    fn from(value: identifiers::AsClientId) -> Self {
        let (user_name, client_id) = value.into_parts();
        Self {
            user_name: Some(user_name.into()),
            client_id: Some(client_id.into()),
        }
    }
}

impl TryFrom<AsClientId> for identifiers::AsClientId {
    type Error = AsClientIdError;

    fn try_from(proto: AsClientId) -> Result<Self, Self::Error> {
        Ok(Self::new(
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
    QualifiedUserName(#[from] QualifiedUserNameError),
}

impl From<AsClientIdError> for Status {
    fn from(e: AsClientIdError) -> Self {
        Status::invalid_argument(format!("invalid AS client id: {e}"))
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
            version => return Err(ClientCredentialCsrError::UnexpectedMlsVersion(version)),
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
    #[error("unexpected MLS version: {0}")]
    UnexpectedMlsVersion(u32),
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
            0 => Ok(messages::MlsInfraVersion::Alpha),
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
            credential_fingerprint: Some(value.signer_fingerprint.into()),
        }
    }
}

impl TryFrom<ClientCredentialPayload> for credentials::ClientCredentialPayload {
    type Error = ClientCredentialPayloadError;

    fn try_from(proto: ClientCredentialPayload) -> Result<Self, Self::Error> {
        let csr = proto.csr.ok_or_missing_field("csr")?.try_into()?;
        let expiration_data = proto.expiration_data.map(TryFrom::try_from).transpose()?;
        let signer_fingerprint = proto
            .credential_fingerprint
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

impl From<ClientCredentialPayloadError> for Status {
    fn from(e: ClientCredentialPayloadError) -> Self {
        Status::invalid_argument(format!("invalid client payload: {e}"))
    }
}

impl From<messages::client_as::ConnectionPackage> for ConnectionPackage {
    fn from(value: messages::client_as::ConnectionPackage) -> Self {
        let (payload, signature) = value.into_parts();
        Self {
            payload: Some(payload.into()),
            signature: Some(signature.into()),
        }
    }
}

impl TryFrom<ConnectionPackage> for messages::client_as_out::ConnectionPackageIn {
    type Error = ConnectionPackageError;

    fn try_from(proto: ConnectionPackage) -> Result<Self, Self::Error> {
        Ok(Self::new(
            proto.payload.ok_or_missing_field("payload")?.try_into()?,
            proto.signature.ok_or_missing_field("signature")?.into(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionPackageError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    Payload(#[from] ConnectionPackagePayloadError),
}

impl From<ConnectionPackageError> for Status {
    fn from(error: ConnectionPackageError) -> Self {
        Status::invalid_argument(format!("invalid connection package: {error}"))
    }
}

impl From<messages::client_as::ConnectionPackageTbs> for ConnectionPackagePayload {
    fn from(value: messages::client_as::ConnectionPackageTbs) -> Self {
        Self {
            protocol_version: Some(value.protocol_version.into()),
            encryption_key: Some(value.encryption_key.into()),
            lifetime: Some(value.lifetime.into()),
            client_credential: Some(value.client_credential.into()),
        }
    }
}

impl TryFrom<ConnectionPackagePayload> for messages::client_as_out::ConnectionPackageTbsIn {
    type Error = ConnectionPackagePayloadError;

    fn try_from(proto: ConnectionPackagePayload) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol_version: proto
                .protocol_version
                .ok_or_missing_field("protocol_version")?
                .try_into()?,
            encryption_key: proto
                .encryption_key
                .ok_or_missing_field("encryption_key")?
                .into(),
            lifetime: proto.lifetime.ok_or_missing_field("lifetime")?.try_into()?,
            client_credential: proto
                .client_credential
                .ok_or_missing_field("client_credential")?
                .try_into()?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionPackagePayloadError {
    #[error(transparent)]
    MissingField(#[from] MissingFieldError<&'static str>),
    #[error(transparent)]
    Version(#[from] UnsupportedMlsVersion),
    #[error(transparent)]
    Lifetime(#[from] ExpirationDataError),
    #[error(transparent)]
    ClientCredential(#[from] ClientCredentialError),
}

impl From<crypto::ConnectionEncryptionKey> for ConnectionEncryptionKey {
    fn from(value: crypto::ConnectionEncryptionKey) -> Self {
        Self {
            bytes: value.into_bytes(),
        }
    }
}

impl From<ConnectionEncryptionKey> for crypto::ConnectionEncryptionKey {
    fn from(proto: ConnectionEncryptionKey) -> Self {
        Self::from_bytes(proto.bytes)
    }
}

impl From<messages::client_as_out::EncryptedUserProfile> for EncryptedUserProfile {
    fn from(value: messages::client_as_out::EncryptedUserProfile) -> Self {
        Self {
            ciphertext: Some(value.into()),
        }
    }
}

impl TryFrom<EncryptedUserProfile> for messages::client_as_out::EncryptedUserProfile {
    type Error = InvalidIndexedCiphertext;

    fn try_from(proto: EncryptedUserProfile) -> Result<Self, Self::Error> {
        proto
            .ciphertext
            .ok_or_missing_field("ciphertext")?
            .try_into()
    }
}
