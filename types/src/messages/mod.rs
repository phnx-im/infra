// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io;

use mls_assist::{
    openmls::prelude::OpenMlsRand, openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::OpenMlsProvider,
};
use serde::{Deserialize, Serialize};
use tls_codec::{DeserializeBytes, TlsDeserializeBytes, TlsSerialize, TlsSize};

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
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
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
///
/// **WARNING**: Only add new variants with new API versions. Do not reuse the API version (variant
/// tag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MlsInfraVersion {
    /// Fallback for unknown versions
    Other(u16) = 0,
    Alpha = 1,
}

impl Serialize for MlsInfraVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MlsInfraVersion::Other(v) => v.serialize(serializer),
            MlsInfraVersion::Alpha => 1u16.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for MlsInfraVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match u16::deserialize(deserializer)? {
            1 => Ok(MlsInfraVersion::Alpha),
            other => Ok(MlsInfraVersion::Other(other)),
        }
    }
}

impl tls_codec::Size for MlsInfraVersion {
    fn tls_serialized_len(&self) -> usize {
        match self {
            MlsInfraVersion::Other(version) => version.tls_serialized_len(),
            MlsInfraVersion::Alpha => 1u16.tls_serialized_len(),
        }
    }
}

impl tls_codec::Serialize for MlsInfraVersion {
    fn tls_serialize<W: io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            MlsInfraVersion::Other(v) => v.tls_serialize(writer),
            MlsInfraVersion::Alpha => 1u16.tls_serialize(writer),
        }
    }
}

impl DeserializeBytes for MlsInfraVersion {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = u16::tls_deserialize_bytes(bytes)?;
        match version {
            1 => Ok((MlsInfraVersion::Alpha, bytes)),
            _ => Ok((MlsInfraVersion::Other(version), bytes)),
        }
    }
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
    AsKeyPackageBatch,
    DsGroupCreation,
    DsGroupOperation,
    QsKeyPackageBatch,
}

#[cfg(test)]
mod tests {
    use tls_codec::Serialize;

    use super::*;

    #[test]
    fn test_mls_infra_version_tls_serde() {
        let version = MlsInfraVersion::Alpha;
        let serialized = version.tls_serialize_detached().unwrap();
        let deserialized = MlsInfraVersion::tls_deserialize_exact_bytes(&serialized).unwrap();
        assert_eq!(deserialized, version);

        let version = MlsInfraVersion::Other(256);
        let serialized = version.tls_serialize_detached().unwrap();
        let deserialized = MlsInfraVersion::tls_deserialize_exact_bytes(&serialized).unwrap();
        assert_eq!(deserialized, version);
    }

    #[test]
    fn test_mls_infra_version_serde() {
        let version = MlsInfraVersion::Alpha;
        let serialized = serde_json::to_string(&version).unwrap();
        let deserialized: MlsInfraVersion = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, version);

        let version = MlsInfraVersion::Other(256);
        let serialized = serde_json::to_string(&version).unwrap();
        let deserialized: MlsInfraVersion = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, version);
    }
}
