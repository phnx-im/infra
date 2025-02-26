// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{CURRENT_VERSION, Entity, Key};
use rusqlite::{OptionalExtension, params};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityRefWrapper, EntityWrapper, KeyRefWrapper};

pub(crate) struct StorablePskBundle<PskBundle: Entity<CURRENT_VERSION>>(PskBundle);

impl<PskBundle: Entity<CURRENT_VERSION>> Storable for StorablePskBundle<PskBundle> {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS psks (
        psk_id BLOB PRIMARY KEY,
        psk_bundle BLOB NOT NULL
    );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(psk) = row.get(0)?;
        Ok(Self(psk))
    }
}

impl<PskBundle: Entity<CURRENT_VERSION>> StorablePskBundle<PskBundle> {
    pub(super) fn load<PskId: Key<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        psk_id: &PskId,
    ) -> Result<Option<PskBundle>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT psk_bundle FROM psks WHERE psk_id = ?1")?;
        stmt.query_row(params![KeyRefWrapper(psk_id)], Self::from_row)
            .map(|x| x.0)
            .optional()
    }
}

pub(super) struct StorablePskBundleRef<'a, PskBundle: Entity<CURRENT_VERSION>>(pub &'a PskBundle);

impl<PskBundle: Entity<CURRENT_VERSION>> StorablePskBundleRef<'_, PskBundle> {
    pub(super) fn store<PskId: Key<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        psk_id: &PskId,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO psks (psk_id, psk_bundle) VALUES (?1, ?2)",
            params![KeyRefWrapper(psk_id), EntityRefWrapper(self.0)],
        )?;
        Ok(())
    }
}

pub(super) struct StorablePskIdRef<'a, PskId: Key<CURRENT_VERSION>>(pub &'a PskId);

impl<PskId: Key<CURRENT_VERSION>> StorablePskIdRef<'_, PskId> {
    pub(super) fn delete(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM psks WHERE psk_id = ?1",
            params![KeyRefWrapper(self.0)],
        )?;
        Ok(())
    }
}
