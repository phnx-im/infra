// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{EncryptionKey as EncryptionKeyTrait, HpkeKeyPair},
    CURRENT_VERSION,
};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityWrapper, KeyRefWrapper, SqliteStorageProviderError};

pub(crate) struct StorableEncryptionKeyPair {
    encryption_key_pair_bytes: Vec<u8>,
}

impl Storable for StorableEncryptionKeyPair {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS encryption_keys (
        public_key BLOB PRIMARY KEY,
        key_pair BLOB NOT NULL,
    )";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let encryption_key_pair_bytes = row.get(0)?;
        Ok(Self {
            encryption_key_pair_bytes,
        })
    }
}

impl StorableEncryptionKeyPair {
    pub(super) fn new<EncryptionKeyPair: HpkeKeyPair<CURRENT_VERSION>>(
        encryption_key_pair: &EncryptionKeyPair,
    ) -> Result<Self, SqliteStorageProviderError> {
        let encryption_key_pair_bytes = serde_json::to_vec(encryption_key_pair)?;
        Ok(Self {
            encryption_key_pair_bytes,
        })
    }

    pub(super) fn store<EncryptionKey: EncryptionKeyTrait<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        public_key: &EncryptionKey,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "INSERT INTO encryption_keys (public_key, key_pair) VALUES (?1, ?2)",
            params![KeyRefWrapper(public_key), self.encryption_key_pair_bytes],
        )?;
        Ok(())
    }

    pub(super) fn load<
        EncryptionKey: EncryptionKeyTrait<CURRENT_VERSION>,
        EncryptionKeyPair: HpkeKeyPair<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        public_key: &EncryptionKey,
    ) -> Result<Option<EncryptionKeyPair>, SqliteStorageProviderError> {
        let row = connection
            .query_row(
                "SELECT key_pair FROM encryption_keys WHERE public_key = ?1",
                params![KeyRefWrapper(public_key)],
                |row| {
                    let EntityWrapper(encryption_key_pair) = row.get(0)?;
                    Ok(encryption_key_pair)
                },
            )
            .optional()?;
        Ok(row)
    }

    pub(super) fn delete<EncryptionKey: EncryptionKeyTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        public_key: &EncryptionKey,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM encryption_keys WHERE public_key = ?1",
            params![KeyRefWrapper(public_key)],
        )?;
        Ok(())
    }
}
