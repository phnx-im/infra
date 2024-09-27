// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{Lifetime, OpenMlsProvider, SignatureScheme},
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{
        random::OpenMlsRand,
        signatures::{Signer, SignerError},
    },
};
#[cfg(feature = "sqlite")]
use rusqlite::{types::ToSqlOutput, ToSql};
use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserializeBytes, TlsSerialize, TlsSize};

use super::{
    infra_credentials::{InfraCredential, InfraCredentialTbs},
    AsCredential, AsIntermediateCredential,
};

#[cfg(feature = "sqlite")]
use crate::codec::PhnxCodec;

use crate::crypto::{
    ear::{keys::SignatureEarKey, EarEncryptable},
    errors::KeyGenerationError,
    signatures::{
        private_keys::{SigningKey, VerifyingKey},
        signable::Signable,
        traits::{SigningKeyBehaviour, VerifyingKeyBehaviour},
        DEFAULT_SIGNATURE_SCHEME,
    },
};

use thiserror::Error;

use super::ClientCredential;

#[derive(Clone, Serialize, Deserialize)]
pub struct AsIntermediateSigningKey {
    signing_key: SigningKey,
    credential: AsIntermediateCredential,
}

impl AsRef<SigningKey> for AsIntermediateSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl SigningKeyBehaviour for AsIntermediateSigningKey {}

impl AsIntermediateSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryAsIntermediateSigningKey,
        credential: AsIntermediateCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key() != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key: prelim_key.into_signing_key(),
            credential,
        })
    }

    pub fn credential(&self) -> &AsIntermediateCredential {
        &self.credential
    }
}

#[derive(Debug, Error)]
pub enum SigningKeyCreationError {
    #[error("Public key mismatch")]
    PublicKeyMismatch,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AsSigningKey {
    signing_key: SigningKey,
    credential: AsCredential,
}

impl AsRef<SigningKey> for AsSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl AsSigningKey {
    pub(super) fn from_private_key_and_credential(
        private_key: SigningKey,
        credential: AsCredential,
    ) -> Self {
        Self {
            signing_key: private_key,
            credential,
        }
    }

    pub fn credential(&self) -> &AsCredential {
        &self.credential
    }
}

impl SigningKeyBehaviour for AsSigningKey {}

#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct AsVerifyingKey(pub(super) VerifyingKey);

impl VerifyingKeyBehaviour for AsVerifyingKey {}

impl AsRef<VerifyingKey> for AsVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct AsIntermediateVerifyingKey(pub(super) VerifyingKey);

impl VerifyingKeyBehaviour for AsIntermediateVerifyingKey {}

impl AsRef<VerifyingKey> for AsIntermediateVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientSigningKey {
    signing_key: SigningKey,
    credential: ClientCredential,
}

impl AsRef<SigningKey> for ClientSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl SigningKeyBehaviour for ClientSigningKey {}

impl ClientSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryClientSigningKey,
        credential: ClientCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key() != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key: prelim_key.into_signing_key(),
            credential,
        })
    }

    pub fn credential(&self) -> &ClientCredential {
        &self.credential
    }
}

#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct ClientVerifyingKey(pub(super) VerifyingKey);

impl VerifyingKeyBehaviour for ClientVerifyingKey {}

impl AsRef<VerifyingKey> for ClientVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfraCredentialSigningKey {
    signing_key: SigningKey,
    credential: InfraCredential,
}

#[cfg(feature = "sqlite")]
impl ToSql for InfraCredentialSigningKey {
    fn to_sql(&self) -> Result<rusqlite::types::ToSqlOutput<'_>, rusqlite::Error> {
        let bytes = PhnxCodec::to_vec(self)?;
        Ok(ToSqlOutput::from(bytes))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for InfraCredentialSigningKey {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let key = PhnxCodec::from_slice(value.as_blob()?)?;
        Ok(key)
    }
}

// 30 days lifetime in seconds
pub(crate) const DEFAULT_INFRA_CREDENTIAL_LIFETIME: u64 = 30 * 24 * 60 * 60;

impl InfraCredentialSigningKey {
    pub fn generate(client_signer: &ClientSigningKey, ear_key: &SignatureEarKey) -> Self {
        let signing_key = SigningKey::generate().unwrap();
        let identity = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        let tbs = InfraCredentialTbs {
            identity,
            expiration_data: Lifetime::new(DEFAULT_INFRA_CREDENTIAL_LIFETIME),
            signature_scheme: DEFAULT_SIGNATURE_SCHEME,
            verifying_key: signing_key.verifying_key().clone().into(),
        };
        let plaintext_credential = tbs.sign(client_signer).unwrap();
        let encrypted_signature = plaintext_credential.signature.encrypt(ear_key).unwrap();
        let credential = InfraCredential::new(
            plaintext_credential.payload.identity,
            plaintext_credential.payload.expiration_data,
            plaintext_credential.payload.signature_scheme,
            plaintext_credential.payload.verifying_key,
            encrypted_signature.tls_serialize_detached().unwrap().into(),
        );
        Self {
            signing_key,
            credential,
        }
    }

    pub fn credential(&self) -> &InfraCredential {
        &self.credential
    }
}

impl SigningKeyBehaviour for InfraCredentialSigningKey {}
impl SigningKeyBehaviour for &InfraCredentialSigningKey {}

impl AsRef<SigningKey> for InfraCredentialSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl Signer for InfraCredentialSigningKey {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        <Self as SigningKeyBehaviour>::sign(self, payload)
            .map_err(|_| SignerError::SigningError)
            .map(|s| s.into_bytes())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        self.credential.signature_scheme()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PreliminaryClientSigningKey(SigningKey);

impl PreliminaryClientSigningKey {
    pub(super) fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub(super) fn into_signing_key(self) -> SigningKey {
        self.0
    }

    pub(super) fn verifying_key(&self) -> ClientVerifyingKey {
        ClientVerifyingKey(self.0.verifying_key().clone())
    }
}

pub struct PreliminaryAsIntermediateSigningKey(SigningKey);

impl PreliminaryAsIntermediateSigningKey {
    pub(super) fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub(super) fn into_signing_key(self) -> SigningKey {
        self.0
    }

    pub(super) fn verifying_key(&self) -> AsIntermediateVerifyingKey {
        AsIntermediateVerifyingKey(self.0.verifying_key().clone())
    }
}
