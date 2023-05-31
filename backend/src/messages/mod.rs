// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::{prelude::OpenMlsRand, test_utils::OpenMlsCryptoProvider},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::crypto::{ear::Ciphertext, RandomnessError};

pub mod client_as;
pub mod client_as_out;
pub mod client_ds;
pub mod client_ds_out;
pub mod client_qs;
pub mod client_qs_out;
pub mod intra_backend;

#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    TlsSerialize,
    TlsDeserialize,
    TlsSize,
    PartialEq,
    Eq,
    Clone,
    Debug,
)]
pub struct FriendshipToken {
    token: Vec<u8>,
}

impl FriendshipToken {
    pub fn random() -> Result<Self, RandomnessError> {
        let token = OpenMlsRustCrypto::default()
            .rand()
            .random_vec(32)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;

        Ok(Self { token })
    }

    pub fn token(&self) -> &[u8] {
        self.token.as_ref()
    }
}

/// Enum encoding the version of the MlsInfra protocol that was used to create
/// the given message.
#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, Clone)]
#[repr(u8)]
pub(crate) enum MlsInfraVersion {
    Alpha,
}

impl Default for MlsInfraVersion {
    fn default() -> Self {
        Self::Alpha
    }
}

// === Queue ===

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QueueMessage {
    pub sequence_number: u64,
    pub ciphertext: EncryptedQueueMessage,
}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EncryptedQueueMessage {
    payload: Ciphertext,
}

impl From<Ciphertext> for EncryptedQueueMessage {
    fn from(payload: Ciphertext) -> Self {
        Self { payload }
    }
}

impl AsRef<Ciphertext> for EncryptedQueueMessage {
    fn as_ref(&self) -> &Ciphertext {
        &self.payload
    }
}
