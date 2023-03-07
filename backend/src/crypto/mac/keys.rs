//! This module contains structs implementing MAC keys that can be used to tag
//! or verify messages.
//! TODO: We could further tighten down type safety by parameterizing the key
//! with the type it's allowed to tag similar to the pattern we use for EAR.

use mls_assist::GroupId;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::crypto::{
    kdf::{keys::InitialClientKdfKey, KdfDerivable},
    secrets::Secret,
};

use super::{traits::MacKey, MAC_KEY_SIZE};

pub type EnqueueAuthenticationKeySecret = Secret<MAC_KEY_SIZE>;

/// A secret that is used to authenticate enqueue requests.
#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema, Clone, Serialize, Deserialize)]
pub struct EnqueueAuthenticationKey {
    key: EnqueueAuthenticationKeySecret,
}

impl From<Secret<MAC_KEY_SIZE>> for EnqueueAuthenticationKey {
    fn from(secret: Secret<MAC_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<MAC_KEY_SIZE>> for EnqueueAuthenticationKey {
    fn as_ref(&self) -> &Secret<MAC_KEY_SIZE> {
        &self.key
    }
}

impl EnqueueAuthenticationKey {
    // TODO: This should be removed at some point
    pub fn dummy_value() -> Self {
        Self {
            key: Secret::random().unwrap(),
        }
    }
}

impl MacKey for EnqueueAuthenticationKey {}

/// A secret allowing the authentication of arbitrary requests to the DS as a
/// member of a given group.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberAuthenticationKey {
    key: Secret<MAC_KEY_SIZE>,
}

impl From<Secret<MAC_KEY_SIZE>> for MemberAuthenticationKey {
    fn from(secret: Secret<MAC_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<MAC_KEY_SIZE>> for MemberAuthenticationKey {
    fn as_ref(&self) -> &Secret<MAC_KEY_SIZE> {
        &self.key
    }
}

impl MacKey for MemberAuthenticationKey {}

impl KdfDerivable<InitialClientKdfKey, GroupId, MAC_KEY_SIZE> for MemberAuthenticationKey {
    const LABEL: &'static str = "mac key member authentication";
}

/// A secret allowing the authentication of requests to delete an QS queue.
#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize)]
pub struct QueueDeletionAuthKey {
    key: Secret<MAC_KEY_SIZE>,
}

impl From<Secret<MAC_KEY_SIZE>> for QueueDeletionAuthKey {
    fn from(secret: Secret<MAC_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<MAC_KEY_SIZE>> for QueueDeletionAuthKey {
    fn as_ref(&self) -> &Secret<MAC_KEY_SIZE> {
        &self.key
    }
}

impl MacKey for QueueDeletionAuthKey {}

/// A secret allowing the authentication of requests to update an QS queue.
#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QueueUpdateAuthKey {
    key: Secret<MAC_KEY_SIZE>,
}

impl From<Secret<MAC_KEY_SIZE>> for QueueUpdateAuthKey {
    fn from(secret: Secret<MAC_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<MAC_KEY_SIZE>> for QueueUpdateAuthKey {
    fn as_ref(&self) -> &Secret<MAC_KEY_SIZE> {
        &self.key
    }
}

impl MacKey for QueueUpdateAuthKey {}
