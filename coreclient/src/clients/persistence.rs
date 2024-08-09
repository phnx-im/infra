// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::AsClientId;
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::{open_phnx_db, Storable};

use super::store::{ClientRecord, ClientRecordState, UserCreationState};

impl Storable for UserCreationState {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS user_creation_state (
            client_id BLOB PRIMARY KEY,
            state BLOB NOT NULL
        )
    ";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let state_bytes = row.get_ref(0)?;
        let state = phnxtypes::codec::from_slice(state_bytes.as_blob()?).map_err(|e| {
            log::error!("Failed to deserialize user creation state: {}", e);
            rusqlite::Error::ToSqlConversionFailure(e.into())
        })?;
        Ok(state)
    }
}

impl UserCreationState {
    pub(super) fn load(
        connection: &rusqlite::Connection,
        client_id: &AsClientId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        connection
            .query_row(
                "SELECT state FROM user_creation_state WHERE client_id = ?1",
                [client_id],
                Self::from_row,
            )
            .optional()
    }

    pub(super) fn store(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let state_bytes = phnxtypes::codec::to_vec(self).map_err(|e| {
            log::error!("Failed to serialize user creation state: {}", e);
            rusqlite::Error::ToSqlConversionFailure(e.into())
        })?;
        connection.execute(
            "INSERT OR REPLACE INTO user_creation_state (client_id, state) VALUES (?1, ?2)",
            params![self.client_id(), &state_bytes],
        )?;
        Ok(())
    }
}

impl Storable for ClientRecord {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS client_record (
            client_id BLOB PRIMARY KEY,
            record_state TEXT NOT NULL CHECK (record_state IN ('in_progress', 'finished'))
        );";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let record_state_str: String = row.get(1)?;
        let client_record_state = match record_state_str.as_str() {
            "in_progress" => ClientRecordState::InProgress,
            "finished" => ClientRecordState::Finished,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        Ok(Self {
            as_client_id: row.get(0)?,
            client_record_state,
        })
    }
}

impl ClientRecord {
    pub fn load_all_from_phnx_db(phnx_db_path: &str) -> Result<Vec<Self>, rusqlite::Error> {
        let connection = open_phnx_db(phnx_db_path)?;
        let mut stmt = connection.prepare("SELECT * FROM client_record")?;
        let client_records = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(client_records)
    }

    pub fn load_all(connection: &rusqlite::Connection) -> Result<Vec<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM client_record")?;
        let client_records = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(client_records)
    }

    pub(super) fn load(
        connection: &rusqlite::Connection,
        client_id: &AsClientId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        connection
            .query_row(
                "SELECT * FROM client_record WHERE client_id = ?1",
                [client_id],
                Self::from_row,
            )
            .optional()
    }

    pub(super) fn store(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let record_state_str = match self.client_record_state {
            ClientRecordState::InProgress => "in_progress",
            ClientRecordState::Finished => "finished",
        };
        connection.execute(
            "INSERT OR REPLACE INTO client_record (client_id, record_state) VALUES (?1, ?2)",
            params![self.as_client_id, record_state_str],
        )?;
        Ok(())
    }
}
