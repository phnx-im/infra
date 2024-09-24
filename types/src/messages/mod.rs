// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::OpenMlsRand, openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::OpenMlsProvider,
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::crypto::{ear::Ciphertext, errors::RandomnessError};

pub mod client_as;
pub mod client_as_out;
pub mod client_ds;
pub mod client_ds_out;
pub mod client_qs;
pub mod client_qs_out;
pub mod push_token;
pub mod welcome_attribution_info;

#[derive(
    Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize, PartialEq, Eq, Clone, Debug,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(transparent))]
pub struct FriendshipToken(Vec<u8>);

#[cfg(feature = "sqlite")]
impl rusqlite::types::ToSql for FriendshipToken {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Blob(self.0.clone()),
        ))
    }
}

#[cfg(feature = "sqlite")]
impl rusqlite::types::FromSql for FriendshipToken {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(value.as_blob()?.to_vec()))
    }
}

impl FriendshipToken {
    pub fn random() -> Result<Self, RandomnessError> {
        let token = OpenMlsRustCrypto::default()
            .rand()
            .random_vec(32)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;

        Ok(Self(token))
    }

    pub fn token(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Enum encoding the version of the MlsInfra protocol that was used to create
/// the given message.
#[derive(
    Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Copy, Serialize, Deserialize,
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
    Clone, Debug, PartialEq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
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
    AsKeyPackageBatch,
    DsGroupCreation,
    DsGroupOperation,
    QsKeyPackageBatch,
}
