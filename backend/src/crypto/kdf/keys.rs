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
use utoipa::ToSchema;

use crate::crypto::{secrets::Secret, RandomnessError};

use super::{traits::KdfKey, KdfDerivable, KdfExtractable, KDF_KEY_SIZE};

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

/// A key that can be derived from a [`RosterExtractedKey`] and subsequently
/// used to derive a [`RosterEarKey`] or as input in the extraction of a new
/// [`RosterExtractedKey`].
/// TODO: I think for a clean key schedule design, we need another derivation
/// step before we can use this as input for an extraction.
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
#[derive(TlsSerialize, TlsSize, TlsDeserializeBytes, Clone, Debug, ToSchema)]
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
    Serialize, Deserialize, Clone, Debug, PartialEq, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct RatchetSecret {
    key: RatchetSecretKey,
}

impl RatchetSecret {
    pub fn random() -> Result<Self, RandomnessError> {
        let key = Secret::random()?;
        Ok(Self { key })
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

pub type FriendshipSecretKey = Secret<KDF_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct FriendshipSecret {
    key: FriendshipSecretKey,
}

impl FriendshipSecret {
    pub fn random() -> Result<Self, RandomnessError> {
        let key = Secret::random()?;
        Ok(Self { key })
    }
}

impl AsRef<Secret<KDF_KEY_SIZE>> for FriendshipSecret {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.key
    }
}

impl KdfKey for FriendshipSecret {
    const ADDITIONAL_LABEL: &'static str = "FriendshipSecret";
}

impl From<Secret<KDF_KEY_SIZE>> for FriendshipSecret {
    fn from(key: Secret<KDF_KEY_SIZE>) -> Self {
        Self { key }
    }
}
