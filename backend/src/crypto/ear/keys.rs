// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs implementing the various keys for EAR
//! throughout the backend. Keys can either provide their own constructors or
//! implement the [`KdfDerivable`] trait to allow derivation from other key.

use mls_assist::openmls::prelude::GroupId;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::crypto::{
    kdf::{
        keys::{InitialClientKdfKey, RatchetSecret, RosterKdfKey},
        KdfDerivable,
    },
    secrets::Secret,
};

use super::{traits::EarKey, AEAD_KEY_SIZE};

pub type GroupStateEarKeySecret = Secret<AEAD_KEY_SIZE>;

/// Key to encrypt/decrypt the roster of the DS group state. Roster keys can be
/// derived either from an initial client KDF key or from a derived roster KDF
/// key.
#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct GroupStateEarKey {
    key: GroupStateEarKeySecret,
}

impl GroupStateEarKey {
    pub(crate) fn as_slice(&self) -> &[u8] {
        self.key.secret.as_slice()
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for GroupStateEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl EarKey for GroupStateEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for GroupStateEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl KdfDerivable<InitialClientKdfKey, GroupId, AEAD_KEY_SIZE> for GroupStateEarKey {
    const LABEL: &'static str = "roster ear key";
}

impl KdfDerivable<RosterKdfKey, GroupId, AEAD_KEY_SIZE> for GroupStateEarKey {
    const LABEL: &'static str = "roster kdf key";
}

pub type DeleteAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type PushTokenEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type FriendshipEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type RatchetKeySecret = Secret<AEAD_KEY_SIZE>;

/// EAR key for the [`PushToken`] structs.
#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema, Serialize, Deserialize)]
pub struct PushTokenEarKey {
    key: PushTokenEarKeySecret,
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

// EAR key for the [`ClientCredential`]s.
#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema, Serialize, Deserialize)]
pub struct FriendshipEarKey {
    key: FriendshipEarKeySecret,
}

impl EarKey for FriendshipEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for FriendshipEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for FriendshipEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type EnqueueAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
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

pub type SignatureEncryptionSecret = Secret<AEAD_KEY_SIZE>;

/// EAR key for the [`PushToken`] structs.
#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema, Serialize, Deserialize)]
pub struct SignatureEncryptionKey {
    key: SignatureEncryptionSecret,
}

impl EarKey for SignatureEncryptionKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for SignatureEncryptionKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for SignatureEncryptionKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}
