// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{codec::PhnxCodec, identifiers::AsClientId};
use rusqlite::{params, types::FromSql, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};
use sqlx::{
    encode::IsNull, error::BoxDynError, query, query_scalar, sqlite::SqliteTypeInfo, Database,
    Decode, Encode, Sqlite, SqlitePool, Type,
};

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

impl Type<Sqlite> for UserCreationState {
    fn type_info() -> SqliteTypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for UserCreationState {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let state = StorableUserCreationStateRef::CurrentVersion(self);
        let bytes = PhnxCodec::to_vec(&state)?;
        <Vec<u8> as Encode<Sqlite>>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for UserCreationState {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as Decode<'r, Sqlite>>::decode(value)?;
        let state = PhnxCodec::from_slice(bytes)?;
        match state {
            StorableUserCreationState::CurrentVersion(state) => Ok(state),
        }
    }
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
            state BLOB NOT NULL
        );";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        row.get(0)
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

    pub(super) async fn load_2(
        db: &SqlitePool,
        client_id: &AsClientId,
    ) -> sqlx::Result<Option<Self>> {
        query_scalar!(
            r#"SELECT state AS "state: _" FROM user_creation_state WHERE client_id = ?"#,
            client_id
        )
        .fetch_optional(db)
        .await
    }

    pub(super) fn store(&self, connection: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO user_creation_state (client_id, state) VALUES (?1, ?2)",
            params![self.client_id(), self],
        )?;
        Ok(())
    }

    pub(super) async fn store_2(&self, db: &SqlitePool) -> sqlx::Result<()> {
        let client_id = self.client_id();
        query!(
            "INSERT OR REPLACE INTO user_creation_state (client_id, state) VALUES (?, ?)",
            client_id,
            self
        )
        .execute(db)
        .await?;
        Ok(())
    }
}

// TODO: This is stored in a different db
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
