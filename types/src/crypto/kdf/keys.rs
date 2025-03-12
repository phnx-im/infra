// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs implementing keys that other keys can be
//! derived from. For keys (or other values) to be derived from one of these
//! keys, the target key (or value) needs to implement the [`KdfDerivable`]
//! trait.

use mls_assist::openmls::prelude::GroupEpoch;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::crypto::{errors::RandomnessError, secrets::Secret};

use super::{KDF_KEY_SIZE, KdfDerivable, KdfExtractable, traits::KdfKey};

/// A secret meant to be injected into the extraction of the new roster kdf key.
#[derive(TlsSerialize, TlsSize, TlsDeserializeBytes, Clone, Debug)]
pub struct RosterKdfInjection {
    secret: Secret<KDF_KEY_SIZE>,
}

impl AsRef<Secret<KDF_KEY_SIZE>> for RosterKdfInjection {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.secret
    }
}

/// A key that can be extracted from a previous [`RosterKdfKey`] and a fresh
/// [`RosterKdfInjection`].
#[derive(Debug)]
pub(crate) struct RosterExtractedKey {
    secret: Secret<KDF_KEY_SIZE>,
}

impl From<Secret<KDF_KEY_SIZE>> for RosterExtractedKey {
    fn from(secret: Secret<KDF_KEY_SIZE>) -> Self {
        Self { secret }
    }
}

impl AsRef<Secret<KDF_KEY_SIZE>> for RosterExtractedKey {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.secret
    }
}

impl KdfKey for RosterExtractedKey {
    const ADDITIONAL_LABEL: &'static str = "roster expanded key";
}

impl KdfExtractable<RosterKdfKey, RosterKdfInjection> for RosterExtractedKey {}

/// A key that can be derived from a `RosterExtractedKey` and subsequently
/// used to derive a `RosterEarKey` or as input in the extraction of a new
/// `RosterExtractedKey`.
// TODO: I think for a clean key schedule design, we need another derivation
// step before we can use this as input for an extraction.
#[derive(TlsSerialize, TlsSize, TlsDeserializeBytes, Clone, Debug)]
pub struct RosterKdfKey {
    key: Secret<KDF_KEY_SIZE>,
}

impl From<Secret<KDF_KEY_SIZE>> for RosterKdfKey {
    fn from(secret: Secret<KDF_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<KDF_KEY_SIZE>> for RosterKdfKey {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.key
    }
}

impl KdfKey for RosterKdfKey {
    const ADDITIONAL_LABEL: &'static str = "roster kdf key";
}

impl KdfDerivable<RosterExtractedKey, GroupEpoch, KDF_KEY_SIZE> for RosterKdfKey {
    const LABEL: &'static str = "roster kdf key";
}

pub type InitialClientKdfKeySecret = Secret<KDF_KEY_SIZE>;

/// A key used to derive further key material for use by the DS when a client
/// joins a group.
#[derive(TlsSerialize, TlsSize, TlsDeserializeBytes, Clone, Debug)]
pub struct InitialClientKdfKey {
    key: InitialClientKdfKeySecret,
}

impl From<Secret<KDF_KEY_SIZE>> for InitialClientKdfKey {
    fn from(secret: Secret<KDF_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl KdfKey for InitialClientKdfKey {
    const ADDITIONAL_LABEL: &'static str = "initial client kdf key";
}

impl AsRef<Secret<KDF_KEY_SIZE>> for InitialClientKdfKey {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.key
    }
}

pub type RatchetSecretKey = Secret<KDF_KEY_SIZE>;

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct RatchetSecret {
    key: RatchetSecretKey,
}

impl RatchetSecret {
    pub fn random() -> Result<Self, RandomnessError> {
        let key = Secret::random()?;
        Ok(Self { key })
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(key: Secret<KDF_KEY_SIZE>) -> Self {
        Self { key }
    }
}

impl AsRef<Secret<KDF_KEY_SIZE>> for RatchetSecret {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.key
    }
}

impl KdfKey for RatchetSecret {
    const ADDITIONAL_LABEL: &'static str = "RatchetSecret";
}

impl From<Secret<KDF_KEY_SIZE>> for RatchetSecret {
    fn from(key: Secret<KDF_KEY_SIZE>) -> Self {
        Self { key }
    }
}

impl KdfDerivable<RatchetSecret, Vec<u8>, KDF_KEY_SIZE> for RatchetSecret {
    const LABEL: &'static str = "RatchetSecret derive";
}

pub type ConnectionKeyKey = Secret<KDF_KEY_SIZE>;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct ConnectionKey {
    key: ConnectionKeyKey,
}

impl sqlx::Type<sqlx::Sqlite> for ConnectionKey {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <ConnectionKeyKey as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for ConnectionKey {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        self.key.encode_by_ref(buf)
    }
}

impl sqlx::Decode<'_, sqlx::Sqlite> for ConnectionKey {
    fn decode(
        value: <sqlx::Sqlite as sqlx::Database>::ValueRef<'_>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let key = ConnectionKeyKey::decode(value)?;
        Ok(Self { key })
    }
}

impl ConnectionKey {
    pub fn random() -> Result<Self, RandomnessError> {
        let key = Secret::random()?;
        Ok(Self { key })
    }
}

impl AsRef<Secret<KDF_KEY_SIZE>> for ConnectionKey {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.key
    }
}

impl KdfKey for ConnectionKey {
    const ADDITIONAL_LABEL: &'static str = "FriendshipSecret";
}

impl From<Secret<KDF_KEY_SIZE>> for ConnectionKey {
    fn from(key: Secret<KDF_KEY_SIZE>) -> Self {
        Self { key }
    }
}
