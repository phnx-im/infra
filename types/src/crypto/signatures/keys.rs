// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{HashType, OpenMlsCrypto, OpenMlsProvider, SignaturePublicKey},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize, VLBytes};

use crate::{
    codec::PhnxCodec,
    crypto::errors::{KeyGenerationError, RandomnessError},
};

use super::{
    private_keys::{generate_signature_keypair, PrivateKey},
    traits::{SigningKey, VerifyingKey},
};

#[derive(Debug)]
pub struct LeafVerifyingKeyRef<'a> {
    verifying_key: &'a SignaturePublicKey,
}

impl<'a> VerifyingKey for LeafVerifyingKeyRef<'a> {}

impl<'a> AsRef<[u8]> for LeafVerifyingKeyRef<'a> {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key.as_slice()
    }
}

impl<'a> From<&'a SignaturePublicKey> for LeafVerifyingKeyRef<'a> {
    fn from(pk_ref: &'a SignaturePublicKey) -> Self {
        Self {
            verifying_key: pk_ref,
        }
    }
}

/// Public signature key known to all clients of a given user. This signature
/// key is used by pseudomnymous clients to prove they belong to a certain
/// pseudonymous user account.
#[derive(Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct UserAuthVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for UserAuthVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for UserAuthVerifyingKey {}

impl UserAuthVerifyingKey {
    pub fn hash(&self) -> UserKeyHash {
        let hash = OpenMlsRustCrypto::default()
            .crypto()
            .hash(HashType::Sha2_256, &self.verifying_key)
            .unwrap_or_default();
        UserKeyHash::new(hash)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAuthSigningKey {
    signing_key: PrivateKey,
    verifying_key: UserAuthVerifyingKey,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for UserAuthSigningKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let bytes = PhnxCodec::to_vec(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Blob(bytes),
        ))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for UserAuthSigningKey {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let key = PhnxCodec::from_slice(value.as_blob()?)?;
        Ok(key)
    }
}

impl UserAuthSigningKey {
    pub fn verifying_key(&self) -> &UserAuthVerifyingKey {
        &self.verifying_key
    }

    pub fn generate() -> Result<Self, KeyGenerationError> {
        let keypair = generate_signature_keypair()?;
        let verifying_key = UserAuthVerifyingKey {
            verifying_key: keypair.1,
        };
        Ok(Self {
            signing_key: keypair.0,
            verifying_key,
        })
    }
}

impl AsRef<PrivateKey> for UserAuthSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl SigningKey for UserAuthSigningKey {}
impl SigningKey for &UserAuthSigningKey {}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
)]
pub struct UserKeyHash {
    pub(super) hash: VLBytes,
}

impl UserKeyHash {
    pub(crate) fn new(hash: Vec<u8>) -> Self {
        Self { hash: hash.into() }
    }
}

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QsClientVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for QsClientVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for QsClientVerifyingKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct QsClientSigningKey {
    signing_key: PrivateKey,
    verifying_key: QsClientVerifyingKey,
}

impl QsClientSigningKey {
    pub fn random() -> Result<Self, RandomnessError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) =
            generate_signature_keypair().map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self {
            signing_key,
            verifying_key: QsClientVerifyingKey { verifying_key },
        })
    }

    pub fn verifying_key(&self) -> &QsClientVerifyingKey {
        &self.verifying_key
    }
}

impl AsRef<PrivateKey> for QsClientSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl SigningKey for QsClientSigningKey {}

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QsUserVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for QsUserVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl QsUserVerifyingKey {
    /// This function is meant to be used only to restore a QsUserSigningKey
    /// from a DB entry.
    pub fn from_bytes(verifying_key: Vec<u8>) -> Self {
        Self { verifying_key }
    }
}

impl VerifyingKey for QsUserVerifyingKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct QsUserSigningKey {
    signing_key: PrivateKey,
    verifying_key: QsUserVerifyingKey,
}

impl QsUserSigningKey {
    pub fn random() -> Result<Self, RandomnessError> {
        let (signing_key, verifying_key) =
            generate_signature_keypair().map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self {
            signing_key,
            verifying_key: QsUserVerifyingKey { verifying_key },
        })
    }

    pub fn verifying_key(&self) -> &QsUserVerifyingKey {
        &self.verifying_key
    }
}

impl AsRef<PrivateKey> for QsUserSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl SigningKey for QsUserSigningKey {}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
pub struct QsVerifyingKey {
    verifying_key: Vec<u8>,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for QsVerifyingKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Borrowed(
            rusqlite::types::ValueRef::Blob(&self.verifying_key),
        ))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for QsVerifyingKey {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self {
            verifying_key: value.as_blob()?.to_vec(),
        })
    }
}

impl QsVerifyingKey {
    pub fn new(verifying_key: Vec<u8>) -> Self {
        Self { verifying_key }
    }
}

impl AsRef<[u8]> for QsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for QsVerifyingKey {}
