// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs implementing the various keys for EAR
//! throughout the backend. Keys can either provide their own constructors or
//! implement the [`KdfDerivable`] trait to allow derivation from other key.

use mls_assist::openmls::prelude::GroupId;

#[cfg(feature = "sqlite")]
use rusqlite::types::FromSql;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    credentials::pseudonymous_credentials::PseudonymousCredentialTbs,
    crypto::{
        errors::RandomnessError,
        kdf::{
            keys::{ConnectionKey, InitialClientKdfKey, RatchetSecret, RosterKdfKey},
            KdfDerivable,
        },
        secrets::Secret,
    },
};

use super::{traits::EarKey, Ciphertext, EarDecryptable, EarEncryptable, AEAD_KEY_SIZE};

pub type GroupStateEarKeySecret = Secret<AEAD_KEY_SIZE>;

/// Key to encrypt/decrypt the roster of the DS group state. Roster keys can be
/// derived either from an initial client KDF key or from a derived roster KDF
/// key.
#[derive(Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct GroupStateEarKey {
    key: GroupStateEarKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for GroupStateEarKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl FromSql for GroupStateEarKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = GroupStateEarKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl GroupStateEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: GroupStateEarKeySecret::random()?,
        })
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for GroupStateEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<AEAD_KEY_SIZE>> for GroupStateEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl EarKey for GroupStateEarKey {}

impl KdfDerivable<InitialClientKdfKey, GroupId, AEAD_KEY_SIZE> for GroupStateEarKey {
    const LABEL: &'static str = "roster ear key";
}

impl KdfDerivable<RosterKdfKey, GroupId, AEAD_KEY_SIZE> for GroupStateEarKey {
    const LABEL: &'static str = "roster kdf key";
}

pub type DeleteAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type PushTokenEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type RatchetKeySecret = Secret<AEAD_KEY_SIZE>;

/// EAR key for the [`crate::messages::push_token::PushToken`] structs.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct PushTokenEarKey {
    key: PushTokenEarKeySecret,
}

impl PushTokenEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: KeyPackageEarKeySecret::random()?,
        })
    }
}

impl EarKey for PushTokenEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for PushTokenEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for PushTokenEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type KeyPackageEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`AddPackage`]s.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct KeyPackageEarKey {
    key: KeyPackageEarKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for KeyPackageEarKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for KeyPackageEarKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = KeyPackageEarKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl KeyPackageEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: KeyPackageEarKeySecret::random()?,
        })
    }
}

impl EarKey for KeyPackageEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for KeyPackageEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for KeyPackageEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type ClientCredentialEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`ClientCredential`]s.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct ClientCredentialEarKey {
    key: ClientCredentialEarKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for ClientCredentialEarKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for ClientCredentialEarKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = ClientCredentialEarKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl ClientCredentialEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: ClientCredentialEarKeySecret::random()?,
        })
    }
}

impl EarKey for ClientCredentialEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for ClientCredentialEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for ClientCredentialEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type EnqueueAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct RatchetKey {
    key: RatchetKeySecret,
}

impl EarKey for RatchetKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for RatchetKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for RatchetKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl KdfDerivable<RatchetSecret, Vec<u8>, AEAD_KEY_SIZE> for RatchetKey {
    const LABEL: &'static str = "RatchetKey";
}

pub type IdentityLinkKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct IdentityLinkKey {
    key: IdentityLinkKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for IdentityLinkKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for IdentityLinkKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = IdentityLinkKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl EarKey for IdentityLinkKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for IdentityLinkKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for IdentityLinkKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl KdfDerivable<ConnectionKey, PseudonymousCredentialTbs, AEAD_KEY_SIZE> for IdentityLinkKey {
    const LABEL: &'static str = "IdentityLinkKey";
}

pub type WelcomeAttributionInfoEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`WelcomeAttributionInfo`]s.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct WelcomeAttributionInfoEarKey {
    key: WelcomeAttributionInfoEarKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for WelcomeAttributionInfoEarKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for WelcomeAttributionInfoEarKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = WelcomeAttributionInfoEarKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl WelcomeAttributionInfoEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: WelcomeAttributionInfoEarKeySecret::random()?,
        })
    }
}

impl EarKey for WelcomeAttributionInfoEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for WelcomeAttributionInfoEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for WelcomeAttributionInfoEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type FriendshipPackageEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`WelcomeAttributionInfo`]s.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct FriendshipPackageEarKey {
    key: FriendshipPackageEarKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for FriendshipPackageEarKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for FriendshipPackageEarKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = FriendshipPackageEarKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl FriendshipPackageEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: FriendshipPackageEarKeySecret::random()?,
        })
    }
}

impl EarKey for FriendshipPackageEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for FriendshipPackageEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for FriendshipPackageEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl EarEncryptable<IdentityLinkWrapperKey, EncryptedIdentityLinkKey> for IdentityLinkKey {}
impl EarDecryptable<IdentityLinkWrapperKey, EncryptedIdentityLinkKey> for IdentityLinkKey {}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes)]
pub struct EncryptedIdentityLinkKey {
    ciphertext: Ciphertext,
}

impl From<Ciphertext> for EncryptedIdentityLinkKey {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl AsRef<Ciphertext> for EncryptedIdentityLinkKey {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

pub type IdentityLinkWrapperKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct IdentityLinkWrapperKey {
    key: IdentityLinkWrapperKeySecret,
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for IdentityLinkWrapperKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.key.to_sql()
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for IdentityLinkWrapperKey {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let key = IdentityLinkWrapperKeySecret::column_result(value)?;
        Ok(Self { key })
    }
}

impl IdentityLinkWrapperKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: IdentityLinkWrapperKeySecret::random()?,
        })
    }
}

impl EarKey for IdentityLinkWrapperKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for IdentityLinkWrapperKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for IdentityLinkWrapperKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}
