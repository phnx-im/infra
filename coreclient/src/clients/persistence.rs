// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use phnxtypes::{codec::PhnxCodec, identifiers::AsClientId};
use rusqlite::{params, types::FromSql, Connection, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};

use crate::utils::persistence::{open_phnx_db, Storable};

use super::store::{ClientRecord, ClientRecordState, UserCreationState};

// When adding a variant to this enum, the new variant must be called
// `CurrentVersion` and the current version must be renamed to `VX`, where `X`
// is the next version number. The content type of the old `CurrentVersion` must
// be renamed and otherwise preserved to ensure backwards compatibility.
#[derive(Serialize, Deserialize)]
enum StorableUserCreationState {
    CurrentVersion(UserCreationState),
}

// Only change this enum in tandem with its non-Ref variant.
#[derive(Serialize)]
enum StorableUserCreationStateRef<'a> {
    CurrentVersion(&'a UserCreationState),
}

impl FromSql for UserCreationState {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let state = PhnxCodec::from_slice(value.as_blob()?)?;
        match state {
            StorableUserCreationState::CurrentVersion(state) => Ok(state),
        }
    }
}

impl ToSql for UserCreationState {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let state = StorableUserCreationStateRef::CurrentVersion(self);
        let bytes = PhnxCodec::to_vec(&state)?;

        Ok(rusqlite::types::ToSqlOutput::from(bytes))
    }
}

impl Storable for UserCreationState {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS user_creation_state (
            client_id BLOB PRIMARY KEY,
            state BLOB NOT NULL,
        );";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        row.get(0)
    }
}

impl UserCreationState {
    pub(super) fn load(
        connection: &Connection,
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

    pub(super) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO user_creation_state (client_id, state) VALUES (?1, ?2)",
            params![self.client_id(), self],
        )?;
        Ok(())
    }
}

impl Storable for ClientRecord {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS client_record (
            client_id BLOB NOT NULL PRIMARY KEY,
            record_state TEXT NOT NULL CHECK (record_state IN ('in_progress', 'finished')),
            created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now', 'utc')),
            is_default BOOLEAN NOT NULL DEFAULT FALSE
        )";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let record_state_str: String = row.get(1)?;
        let client_record_state = match record_state_str.as_str() {
            "in_progress" => ClientRecordState::InProgress,
            "finished" => ClientRecordState::Finished,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        let created_at: DateTime<Utc> = row.get(2)?;
        let is_default: bool = row.get(3)?;
        Ok(Self {
            as_client_id: row.get(0)?,
            client_record_state,
            created_at,
            is_default,
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

    pub fn load_all(connection: &Connection) -> Result<Vec<Self>, rusqlite::Error> {
        let mut stmt = connection.prepare("SELECT * FROM client_record")?;
        let client_records = stmt
            .query_map([], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(client_records)
    }

    pub(super) fn load(
        connection: &Connection,
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

    pub(super) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        let record_state_str = match self.client_record_state {
            ClientRecordState::InProgress => "in_progress",
            ClientRecordState::Finished => "finished",
        };
        connection.execute(
            "INSERT OR REPLACE INTO client_record
            (client_id, record_state, created_at, is_default)
            VALUES (?1, ?2, ?3, ?4)",
            params![
                self.as_client_id,
                record_state_str,
                self.created_at,
                self.is_default,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn delete(connection: &Connection, client_id: &AsClientId) -> rusqlite::Result<()> {
        connection.execute("DELETE FROM client_record WHERE ?", params![client_id])?;
        Ok(())
    }
}
