// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{Lifetime, OpenMlsProvider, SignaturePublicKey, SignatureScheme},
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
    AsCredential, AsIntermediateCredential, PreliminaryAsSigningKey,
};

use crate::{
    codec::DefaultCodec,
    crypto::{
        ear::{keys::SignatureEarKey, EarEncryptable},
        signatures::{
            private_keys::{generate_signature_keypair, PrivateKey},
            signable::Signable,
            traits::{SigningKey, VerifyingKey},
            DEFAULT_SIGNATURE_SCHEME,
        },
    },
};

use thiserror::Error;

use super::{ClientCredential, PreliminaryClientSigningKey};

#[derive(Clone, Serialize, Deserialize)]
pub struct AsIntermediateSigningKey {
    signing_key: PrivateKey,
    credential: AsIntermediateCredential,
}

impl AsRef<PrivateKey> for AsIntermediateSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl SigningKey for AsIntermediateSigningKey {}

impl AsIntermediateSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryAsSigningKey,
        credential: AsIntermediateCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key != credential.verifying_key() {
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
    signing_key: PrivateKey,
    credential: AsCredential,
}

impl AsRef<PrivateKey> for AsSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl AsSigningKey {
    pub(super) fn from_private_key_and_credential(
        private_key: PrivateKey,
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

impl SigningKey for AsSigningKey {}

#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct AsVerifyingKey {
    verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for AsVerifyingKey {}

impl AsRef<[u8]> for AsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

impl From<Vec<u8>> for AsVerifyingKey {
    fn from(value: Vec<u8>) -> Self {
        Self {
            verifying_key_bytes: value.into(),
        }
    }
}

#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct AsIntermediateVerifyingKey {
    pub(super) verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for AsIntermediateVerifyingKey {}

impl AsRef<[u8]> for AsIntermediateVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientSigningKey {
    signing_key: PrivateKey,
    credential: ClientCredential,
}

impl AsRef<PrivateKey> for ClientSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl SigningKey for ClientSigningKey {}

impl ClientSigningKey {
    pub fn from_prelim_key(
        prelim_key: PreliminaryClientSigningKey,
        credential: ClientCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key != credential.verifying_key() {
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
pub struct ClientVerifyingKey {
    pub(super) verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for ClientVerifyingKey {}

impl AsRef<[u8]> for ClientVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfraCredentialSigningKey {
    signing_key: PrivateKey,
    credential: InfraCredential,
}

#[cfg(feature = "sqlite")]
impl ToSql for InfraCredentialSigningKey {
    fn to_sql(&self) -> Result<rusqlite::types::ToSqlOutput<'_>, rusqlite::Error> {
        let bytes = DefaultCodec::to_vec(self).map_err(|e| {
            tracing::error!("Error serializing InfraCredentialSigningKey: {:?}", e);
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;
        Ok(ToSqlOutput::from(bytes))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for InfraCredentialSigningKey {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        DefaultCodec::from_slice(bytes).map_err(|e| {
            tracing::error!("Error deserializing InfraCredentialSigningKey: {:?}", e);
            rusqlite::types::FromSqlError::Other(Box::new(e))
        })
    }
}

// 30 days lifetime in seconds
pub(crate) const DEFAULT_INFRA_CREDENTIAL_LIFETIME: u64 = 30 * 24 * 60 * 60;

impl InfraCredentialSigningKey {
    pub fn generate(client_signer: &ClientSigningKey, ear_key: &SignatureEarKey) -> Self {
        let keypair = generate_signature_keypair().unwrap();
        let identity = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        let tbs = InfraCredentialTbs {
            identity,
            expiration_data: Lifetime::new(DEFAULT_INFRA_CREDENTIAL_LIFETIME),
            signature_scheme: DEFAULT_SIGNATURE_SCHEME,
            verifying_key: keypair.1.clone().into(),
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
            signing_key: keypair.0,
            credential,
        }
    }

    pub fn credential(&self) -> &InfraCredential {
        &self.credential
    }
}

impl SigningKey for InfraCredentialSigningKey {}
impl SigningKey for &InfraCredentialSigningKey {}

impl AsRef<PrivateKey> for InfraCredentialSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl Signer for InfraCredentialSigningKey {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        <Self as SigningKey>::sign(self, payload)
            .map_err(|_| SignerError::SigningError)
            .map(|s| s.into_bytes())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        self.credential.signature_scheme()
    }
}
