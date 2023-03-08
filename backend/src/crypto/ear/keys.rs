//! This module contains structs implementing the various keys for EAR
//! throughout the backend. Keys can either provide their own constructors or
//! implement the [`KdfDerivable`] trait to allow derivation from other key.

use mls_assist::GroupId;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::crypto::{
    kdf::{
        keys::{InitialClientKdfKey, RosterKdfKey},
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
    const LABEL: &'static str = "roster ear key";
}

pub type DeleteAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type PushTokenEarKeySecret = Secret<AEAD_KEY_SIZE>;

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

pub type EnqueueAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;
