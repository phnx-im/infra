// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;

use mls_assist::{
    openmls::prelude::{KeyPackage, KeyPackageIn, OpenMlsRand},
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::OpenMlsProvider,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize, TlsVarInt};

use crate::crypto::{
    ear::{Ciphertext, EarDecryptable, EarEncryptable, keys::KeyPackageEarKey},
    errors::RandomnessError,
};

pub mod client_as;
pub mod client_as_out;
pub mod client_ds;
pub mod client_ds_out;
pub mod client_qs;
pub mod client_qs_out;
pub mod push_token;
pub mod welcome_attribution_info;

#[derive(
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct FriendshipToken(Vec<u8>);

impl FriendshipToken {
    pub fn random() -> Result<Self, RandomnessError> {
        let token = OpenMlsRustCrypto::default()
            .rand()
            .random_vec(32)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;

        Ok(Self(token))
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(token: Vec<u8>) -> Self {
        Self(token)
    }

    pub fn token(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Enum encoding the version of the MlsInfra protocol that was used to create
/// the given message.
#[derive(
    Debug,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]
#[repr(u8)]
pub enum MlsInfraVersion {
    Alpha,
}

impl Default for MlsInfraVersion {
    fn default() -> Self {
        Self::Alpha
    }
}

// === Queue ===

#[derive(
    Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QueueMessage {
    pub sequence_number: u64,
    pub ciphertext: Ciphertext,
}

#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct EncryptedQsQueueMessage {
    payload: Ciphertext,
}

impl From<Ciphertext> for EncryptedQsQueueMessage {
    fn from(payload: Ciphertext) -> Self {
        Self { payload }
    }
}

impl AsRef<Ciphertext> for EncryptedQsQueueMessage {
    fn as_ref(&self) -> &Ciphertext {
        &self.payload
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct EncryptedAsQueueMessage {
    payload: Ciphertext,
}

impl From<Ciphertext> for EncryptedAsQueueMessage {
    fn from(payload: Ciphertext) -> Self {
        Self { payload }
    }
}

impl AsRef<Ciphertext> for EncryptedAsQueueMessage {
    fn as_ref(&self) -> &Ciphertext {
        &self.payload
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
#[repr(u8)]
pub enum AsTokenType {
    AsEnqueue,
    DsGroupCreation,
    DsGroupOperation,
}

/// Ciphertext that contains a KeyPackage and an intermediary client certficate.
/// TODO: do we want a key committing scheme here?
#[derive(
    Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize, sqlx::Type,
)]
#[sqlx(transparent)]
pub struct QsEncryptedKeyPackage(Ciphertext);

impl AsRef<Ciphertext> for QsEncryptedKeyPackage {
    fn as_ref(&self) -> &Ciphertext {
        &self.0
    }
}

impl From<Ciphertext> for QsEncryptedKeyPackage {
    fn from(ctxt: Ciphertext) -> Self {
        Self(ctxt)
    }
}

impl EarDecryptable<KeyPackageEarKey, QsEncryptedKeyPackage> for KeyPackageIn {}
impl EarEncryptable<KeyPackageEarKey, QsEncryptedKeyPackage> for KeyPackage {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion(TlsVarInt);

impl ApiVersion {
    pub const fn new(version: u64) -> Option<Self> {
        // Note: At the moment of writing, Option::map is not a const fn.
        match TlsVarInt::new(version) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    pub const fn value(&self) -> u64 {
        self.0.value()
    }

    pub(crate) const fn from_tls_value(value: TlsVarInt) -> Self {
        Self(value)
    }

    pub const fn tls_value(&self) -> TlsVarInt {
        self.0
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

#[cfg(test)]
mod test {
    use crate::codec::PhnxCodec;

    use super::*;

    #[test]
    fn test_queue_message_serde_codec() {
        let message = QueueMessage {
            sequence_number: 1,
            ciphertext: Ciphertext::dummy(),
        };
        insta::assert_binary_snapshot!(".cbor", PhnxCodec::to_vec(&message).unwrap());
    }

    #[test]
    fn test_queue_message_serde_json() {
        let message = QueueMessage {
            sequence_number: 1,
            ciphertext: Ciphertext::dummy(),
        };
        insta::assert_json_snapshot!(message);
    }
}
