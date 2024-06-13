// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{PskBundle as PskBundleTrait, PskId as PskIdTrait},
    CURRENT_VERSION,
};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{
    EntityRefWrapper, EntityWrapper, KeyRefWrapper, SqliteStorageProviderError,
};

pub(crate) struct StorablePskBundle {}

impl Storable for StorablePskBundle {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS psks (
        psk_id BLOB PRIMARY KEY,
        psk_bundle BLOB NOT NULL,
    )";

    fn from_row(_row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Err(rusqlite::Error::InvalidQuery)
    }
}

impl StorablePskBundle {
    pub(super) fn store<
        PskId: PskIdTrait<CURRENT_VERSION>,
        PskBundle: PskBundleTrait<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        psk_id: &PskId,
        psk: &PskBundle,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "INSERT INTO psks (psk_id, psk_bundle) VALUES (?1, ?2)",
            params![KeyRefWrapper(psk_id), EntityRefWrapper(psk)],
        )?;
        Ok(())
    }

    pub(super) fn load<
        PskId: PskIdTrait<CURRENT_VERSION>,
        PskBundle: PskBundleTrait<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        psk_id: &PskId,
    ) -> Result<Option<PskBundle>, SqliteStorageProviderError> {
        let mut stmt = connection.prepare("SELECT psk_bundle FROM psks WHERE psk_id = ?1")?;
        let psk_bundle = stmt
            .query_row(params![KeyRefWrapper(psk_id)], |row| {
                let EntityWrapper(psk) = row.get(0)?;
                Ok(psk)
            })
            .optional()?;
        Ok(psk_bundle)
    }

    pub(super) fn delete<PskId: PskIdTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        psk_id: &PskId,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM psks WHERE psk_id = ?1",
            params![KeyRefWrapper(psk_id)],
        )?;
        Ok(())
    }
}
