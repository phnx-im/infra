// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use phnxtypes::{codec::PhnxCodec, identifiers::AsClientId};
use rusqlite::{types::FromSql, ToSql};
use serde::{Deserialize, Serialize};
use sqlx::{
    encode::IsNull, error::BoxDynError, query, query_as, query_scalar, sqlite::SqliteTypeInfo,
    Database, Decode, Encode, Sqlite, SqliteExecutor, Type,
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
            state BLOB NOT NULL,
            created_at DATETIME NOT NULL
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        row.get(0)
    }
}

impl UserCreationState {
    pub(super) async fn load(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<Option<Self>> {
        query_scalar!(
            r#"SELECT state AS "state: _"
            FROM user_creation_state WHERE client_id = ?1"#,
            client_id,
        )
        .fetch_optional(executor)
        .await
    }

    pub(super) async fn store(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let client_id = self.client_id();
        query!(
            "INSERT OR REPLACE INTO user_creation_state (client_id, state) VALUES (?, ?)",
            client_id,
            self
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

// TODO: This is stored in a different db
impl Storable for ClientRecord {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS client_record (
            client_id BLOB NOT NULL PRIMARY KEY,
            record_state TEXT NOT NULL CHECK (record_state IN ('in_progress', 'finished')),
            created_at DATETIME NOT NULL,
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

impl Type<Sqlite> for ClientRecordState {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ClientRecordState {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(self.as_str(), buf)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid ClientRecordState: {state}")]
struct InvalidClientRecordState {
    state: String,
}

impl<'r> Decode<'r, Sqlite> for ClientRecordState {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let state: &str = Decode::<Sqlite>::decode(value)?;
        Self::from_str(state).ok_or_else(|| -> BoxDynError {
            Box::new(InvalidClientRecordState {
                state: state.to_string(),
            })
        })
    }
}

impl ClientRecord {
    pub async fn load_all_from_phnx_db(phnx_db_path: &str) -> sqlx::Result<Vec<Self>> {
        let pool = open_phnx_db(phnx_db_path).await?;
        Self::load_all(&pool).await
    }

    pub async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        query_as!(
            ClientRecord,
            r#"
            SELECT
                client_id AS "as_client_id: _",
                record_state AS "client_record_state: _",
                created_at AS "created_at: _",
                is_default AS "is_default: _"
            FROM client_record"#
        )
        .fetch_all(executor)
        .await
    }

    pub(super) async fn load(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            ClientRecord,
            r#"SELECT
                client_id AS "as_client_id: _",
                record_state AS "client_record_state: _",
                created_at AS "created_at: _",
                is_default AS "is_default: _"
            FROM client_record WHERE client_id = ?"#,
            client_id
        )
        .fetch_optional(executor)
        .await
    }

    pub(super) async fn store(&self, executor: impl SqliteExecutor<'_>) -> sqlx::Result<()> {
        let record_state_str = match self.client_record_state {
            ClientRecordState::InProgress => "in_progress",
            ClientRecordState::Finished => "finished",
        };
        query!(
            "INSERT OR REPLACE INTO client_record
            (client_id, record_state, created_at, is_default)
            VALUES (?1, ?2, ?3, ?4)",
            self.as_client_id,
            record_state_str,
            self.created_at,
            self.is_default,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn set_default(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE client_record SET is_default = (client_id == ?)",
            client_id,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<()> {
        query!("DELETE FROM client_record WHERE client_id = ?", client_id)
            .execute(executor)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use super::*;

    fn test_client_record() -> ClientRecord {
        let id = Uuid::new_v4();
        let client_id = AsClientId::new("{id}@localhost".parse().unwrap(), id);
        ClientRecord {
            as_client_id: client_id.clone(),
            client_record_state: ClientRecordState::Finished,
            created_at: Utc::now(),
            is_default: false,
        }
    }

    #[sqlx::test]
    async fn persistence(pool: SqlitePool) -> anyhow::Result<()> {
        let mut alice_record = test_client_record();
        let mut bob_record = test_client_record();

        // Storing and loading client records works
        alice_record.store(&pool).await?;
        bob_record.store(&pool).await?;
        let records = ClientRecord::load_all(&pool).await?;
        assert_eq!(records, [alice_record.clone(), bob_record.clone()]);

        // Set default to alice set alice is_default
        alice_record.is_default = true;
        ClientRecord::set_default(&pool, &alice_record.as_client_id).await?;
        let records = ClientRecord::load_all(&pool).await?;
        assert_eq!(records, [alice_record.clone(), bob_record.clone()]);

        // Set default to bob clears alice is_default
        alice_record.is_default = false;
        bob_record.is_default = true;
        ClientRecord::set_default(&pool, &bob_record.as_client_id).await?;
        let records = ClientRecord::load_all(&pool).await?;
        assert_eq!(records, [alice_record.clone(), bob_record.clone()]);

        // Delete client records
        ClientRecord::delete(&pool, &alice_record.as_client_id).await?;
        ClientRecord::delete(&pool, &bob_record.as_client_id).await?;
        let records = ClientRecord::load_all(&pool).await?;
        assert_eq!(records, []);

        Ok(())
    }
}
