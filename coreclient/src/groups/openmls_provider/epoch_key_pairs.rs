// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{EpochKey as EpochKeyTrait, GroupId as GroupIdTrait, HpkeKeyPair},
    CURRENT_VERSION,
};
use rusqlite::params;

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityWrapper, KeyRefWrapper, SqliteStorageProviderError};

pub(crate) struct StorableEpochKeyPairs {
    key_pair_bytes: Vec<u8>,
}

impl Storable for StorableEpochKeyPairs {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS epoch_keys_pairs (
        group_id BLOB NOT NULL,
        epoch_id BLOB NOT NULL,
        leaf_index INTEGER NOT NULL,
        key_pairs BLOB NOT NULL,
        PRIMARY KEY (group_id, epoch_id, leaf_index)
    )";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let key_pair_bytes = row.get(0)?;
        Ok(Self { key_pair_bytes })
    }
}

impl StorableEpochKeyPairs {
    pub(super) fn new<EpochKeyPairs: HpkeKeyPair<CURRENT_VERSION>>(
        key_pairs: &[EpochKeyPairs],
    ) -> Result<Self, SqliteStorageProviderError> {
        let key_pair_bytes = serde_json::to_vec(key_pairs)?;
        Ok(Self { key_pair_bytes })
    }

    pub(super) fn store<
        GroupId: GroupIdTrait<CURRENT_VERSION>,
        EpochKey: EpochKeyTrait<CURRENT_VERSION>,
    >(
        &self,
        connection: &rusqlite::Connection,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "INSERT INTO epoch_keys_pairs (group_id, epoch_id, leaf_index, key_pairs) VALUES (?1, ?2, ?3, ?4)",
            params![KeyRefWrapper(group_id), KeyRefWrapper(epoch_id), leaf_index, self.key_pair_bytes],
        )?;
        Ok(())
    }

    pub(super) fn load<
        GroupId: GroupIdTrait<CURRENT_VERSION>,
        EpochKey: EpochKeyTrait<CURRENT_VERSION>,
        EpochKeyPairs: HpkeKeyPair<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> Result<Vec<EpochKeyPairs>, SqliteStorageProviderError> {
        let mut stmt = connection.prepare(
            "SELECT key_pairs FROM epoch_keys_pairs WHERE group_id = ?1 AND epoch_id = ?2 AND leaf_index = ?3",
        )?;
        let key_pairs = stmt
            .query_map(
                params![KeyRefWrapper(group_id), KeyRefWrapper(epoch_id), leaf_index],
                |row| {
                    let EntityWrapper(key_pairs) = row.get(0)?;
                    Ok(key_pairs)
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(key_pairs)
    }

    pub(super) fn delete<
        GroupId: GroupIdTrait<CURRENT_VERSION>,
        EpochKey: EpochKeyTrait<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        group_id: &GroupId,
        epoch_id: &EpochKey,
        leaf_index: u32,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM epoch_keys_pairs WHERE group_id = ?1 AND epoch_id = ?2 AND leaf_index = ?3",
            params![KeyRefWrapper(group_id), KeyRefWrapper(epoch_id), leaf_index],
        )?;
        Ok(())
    }
}
