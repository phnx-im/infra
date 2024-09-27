// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, CURRENT_VERSION};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityRefWrapper, EntityWrapper, KeyRefWrapper};

pub(crate) struct StorableEncryptionKeyPair<EncryptionKeyPair: Entity<CURRENT_VERSION>>(
    pub EncryptionKeyPair,
);

impl<EncryptionKeyPair: Entity<CURRENT_VERSION>> Storable
    for StorableEncryptionKeyPair<EncryptionKeyPair>
{
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS encryption_keys (
        public_key BLOB PRIMARY KEY,
        key_pair BLOB NOT NULL
    );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(encryption_key_pair) = row.get(0)?;
        Ok(Self(encryption_key_pair))
    }
}

impl<EncryptionKeyPair: Entity<CURRENT_VERSION>> StorableEncryptionKeyPair<EncryptionKeyPair> {
    pub(super) fn load<EncryptionKey: Key<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        public_key: &EncryptionKey,
    ) -> Result<Option<EncryptionKeyPair>, rusqlite::Error> {
        connection
            .query_row(
                "SELECT key_pair FROM encryption_keys WHERE public_key = ?1",
                params![KeyRefWrapper(public_key)],
                Self::from_row,
            )
            .map(|x| x.0)
            .optional()
    }
}

pub(crate) struct StorableEncryptionKeyPairRef<'a, EncryptionKeyPair: Entity<CURRENT_VERSION>>(
    pub &'a EncryptionKeyPair,
);

impl<'a, EncryptionKeyPair: Entity<CURRENT_VERSION>>
    StorableEncryptionKeyPairRef<'a, EncryptionKeyPair>
{
    pub(super) fn store<EncryptionKey: Key<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        public_key: &EncryptionKey,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO encryption_keys (public_key, key_pair) VALUES (?1, ?2)",
            params![KeyRefWrapper(public_key), EntityRefWrapper(self.0)],
        )?;
        Ok(())
    }
}

pub(crate) struct StorableEncryptionPublicKeyRef<'a, EncryptionPublicKey: Key<CURRENT_VERSION>>(
    pub &'a EncryptionPublicKey,
);

impl<'a, EncryptionPublicKey: Key<CURRENT_VERSION>>
    StorableEncryptionPublicKeyRef<'a, EncryptionPublicKey>
{
    pub(super) fn delete(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM encryption_keys WHERE public_key = ?1",
            params![KeyRefWrapper(self.0)],
        )?;
        Ok(())
    }
}
