// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, CURRENT_VERSION};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{
    EntitySliceWrapper, EntityVecWrapper, KeyRefWrapper, StorableGroupIdRef,
};

impl<EpochKeyPairs: Entity<CURRENT_VERSION>> Storable for StorableEpochKeyPairs<EpochKeyPairs> {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS epoch_keys_pairs (
        group_id BLOB NOT NULL,
        epoch_id BLOB NOT NULL,
        leaf_index INTEGER NOT NULL,
        key_pairs BLOB NOT NULL,
        PRIMARY KEY (group_id, epoch_id, leaf_index)
    );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityVecWrapper(key_pairs) = row.get(0)?;
        Ok(Self(key_pairs))
    }
}

pub(crate) struct StorableEpochKeyPairs<EpochKeyPairs: Entity<CURRENT_VERSION>>(
    pub Vec<EpochKeyPairs>,
);

impl<EpochKeyPairs: Entity<CURRENT_VERSION>> StorableEpochKeyPairs<EpochKeyPairs> {
    pub(super) fn load<GroupId: Key<CURRENT_VERSION>, EpochKey: Key<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> Result<Vec<EpochKeyPairs>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT key_pairs FROM epoch_keys_pairs WHERE group_id = ?1 AND epoch_id = ?2 AND leaf_index = ?3",
        )?;
        let result = stmt
            .query_row(
                params![KeyRefWrapper(group_id), KeyRefWrapper(epoch_id), leaf_index],
                |row| Self::from_row(row).map(|x| x.0),
            )
            .optional()?
            .unwrap_or_default();
        Ok(result)
    }
}

pub(super) struct StorableEpochKeyPairsRef<'a, EpochKeyPairs: Entity<CURRENT_VERSION>>(
    pub &'a [EpochKeyPairs],
);

impl<EpochKeyPairs: Entity<CURRENT_VERSION>> StorableEpochKeyPairsRef<'_, EpochKeyPairs> {
    pub(super) fn store<GroupId: Key<CURRENT_VERSION>, EpochKey: Key<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO epoch_keys_pairs (group_id, epoch_id, leaf_index, key_pairs) VALUES (?1, ?2, ?3, ?4)",
            params![KeyRefWrapper(group_id), KeyRefWrapper(epoch_id), leaf_index, EntitySliceWrapper(self.0)],
        )?;
        Ok(())
    }
}

impl<GroupId: Key<CURRENT_VERSION>> StorableGroupIdRef<'_, GroupId> {
    pub(super) fn delete_epoch_key_pair<EpochKey: Key<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        epoch_key: &EpochKey,
        leaf_index: u32,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM epoch_keys_pairs WHERE group_id = ?1 AND epoch_id = ?2 AND leaf_index = ?3",
            params![KeyRefWrapper(self.0), KeyRefWrapper(epoch_key), leaf_index],
        )?;
        Ok(())
    }
}
