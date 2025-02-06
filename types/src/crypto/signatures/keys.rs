// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{HashType, OpenMlsCrypto, OpenMlsProvider, SignaturePublicKey},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize, VLBytes};

#[cfg(feature = "sqlite")]
use crate::codec::PhnxCodec;

use crate::crypto::errors::KeyGenerationError;

use super::{
    private_keys::{SigningKey, VerifyingKey},
    traits::{SigningKeyBehaviour, VerifyingKeyBehaviour},
};

#[derive(Debug)]
pub struct LeafVerifyingKey(VerifyingKey);

impl VerifyingKeyBehaviour for LeafVerifyingKey {}

impl AsRef<VerifyingKey> for LeafVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl From<&SignaturePublicKey> for LeafVerifyingKey {
    fn from(pk_ref: &SignaturePublicKey) -> Self {
        Self(pk_ref.clone().into())
    }
}

/// Public signature key known to all clients of a given user. This signature
/// key is used by pseudomnymous clients to prove they belong to a certain
/// pseudonymous user account.
#[derive(Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone)]
pub struct UserAuthVerifyingKey(VerifyingKey);

impl AsRef<VerifyingKey> for UserAuthVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl VerifyingKeyBehaviour for UserAuthVerifyingKey {}

impl UserAuthVerifyingKey {
    pub fn hash(&self) -> UserKeyHash {
        let hash = OpenMlsRustCrypto::default()
            .crypto()
            .hash(HashType::Sha2_256, self.0.as_slice())
            .unwrap_or_default();
        UserKeyHash::new(hash)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAuthSigningKey(SigningKey);

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
    pub fn verifying_key(&self) -> UserAuthVerifyingKey {
        UserAuthVerifyingKey(self.0.verifying_key().clone())
    }

    pub fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }
}

impl AsRef<SigningKey> for UserAuthSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

impl super::traits::SigningKeyBehaviour for UserAuthSigningKey {}
impl super::traits::SigningKeyBehaviour for &UserAuthSigningKey {}

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
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsClientVerifyingKey(VerifyingKey);

impl QsClientVerifyingKey {
    #[cfg(test)]
    pub(crate) fn new_for_test(verifying_key: VerifyingKey) -> Self {
        Self(verifying_key)
    }
}

impl AsRef<VerifyingKey> for QsClientVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl VerifyingKeyBehaviour for QsClientVerifyingKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct QsClientSigningKey(SigningKey);

impl QsClientSigningKey {
    pub fn random() -> Result<Self, KeyGenerationError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub fn verifying_key(&self) -> QsClientVerifyingKey {
        QsClientVerifyingKey(self.0.verifying_key().clone())
    }
}

impl AsRef<SigningKey> for QsClientSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

impl super::traits::SigningKeyBehaviour for QsClientSigningKey {}

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsUserVerifyingKey(VerifyingKey);

impl QsUserVerifyingKey {
    #[cfg(test)]
    pub(crate) fn new_for_test(verifying_key: VerifyingKey) -> Self {
        Self(verifying_key)
    }
}

impl AsRef<VerifyingKey> for QsUserVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl VerifyingKeyBehaviour for QsUserVerifyingKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct QsUserSigningKey(SigningKey);

impl QsUserSigningKey {
    pub fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub fn verifying_key(&self) -> QsUserVerifyingKey {
        QsUserVerifyingKey(self.0.verifying_key().clone())
    }
}

impl AsRef<SigningKey> for QsUserSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

impl SigningKeyBehaviour for QsUserSigningKey {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct QsSigningKey(SigningKey);

impl QsSigningKey {
    pub fn generate() -> Result<Self, KeyGenerationError> {
        let signing_key = SigningKey::generate()?;
        Ok(Self(signing_key))
    }

    pub fn verifying_key(&self) -> QsVerifyingKey {
        QsVerifyingKey(self.0.verifying_key().clone())
    }
}

impl AsRef<SigningKey> for QsSigningKey {
    fn as_ref(&self) -> &SigningKey {
        &self.0
    }
}

impl SigningKeyBehaviour for QsSigningKey {}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct QsVerifyingKey(VerifyingKey);

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for QsVerifyingKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for QsVerifyingKey {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(VerifyingKey::column_result(value)?))
    }
}

impl From<VerifyingKey> for QsVerifyingKey {
    fn from(key: VerifyingKey) -> Self {
        Self(key)
    }
}

impl AsRef<VerifyingKey> for QsVerifyingKey {
    fn as_ref(&self) -> &VerifyingKey {
        &self.0
    }
}

impl VerifyingKeyBehaviour for QsVerifyingKey {}
