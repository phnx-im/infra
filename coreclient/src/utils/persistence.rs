// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::auth_service::AsClientId;
use rusqlite::{named_params, params, Connection, Row, ToSql};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

pub(crate) fn db_path(as_client_id: &AsClientId) -> String {
    format!("{}.db", as_client_id)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum DataType {
    KeyStoreValue,
    Contact,
    PartialContact,
    Conversation,
    MlsGroup,
    Message,
    AsCredential,
    AsIntermediateCredential,
    LeafKeys,
    QsVerifyingKey,
    QueueRatchet,
    SequenceNumber,
    ClientData,
}

impl ToSql for DataType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let data_type_str = self.to_sql_key();
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Text(data_type_str),
        ))
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub(crate) trait PersistenceKey {
    fn to_sql_key(&self) -> String;
}

impl<T: std::fmt::Display> PersistenceKey for T {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

pub(crate) trait Persistable<'a>: Sized {
    type Key: PersistenceKey + std::fmt::Debug;
    type SecondaryKey: PersistenceKey + std::fmt::Debug;
    type Payload: Serialize + DeserializeOwned;
    const DATA_TYPE: DataType;

    fn key(&self) -> &Self::Key;
    fn secondary_key(&self) -> &Self::SecondaryKey;
    fn connection(&self) -> &Connection;
    fn payload(&self) -> &Self::Payload;
    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self;

    fn create_table(conn: &rusqlite::Connection) -> Result<(), PersistenceError> {
        let table_name = Self::DATA_TYPE.to_sql_key();
        let statement_str = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                rowid INTEGER PRIMARY KEY,
                primary_key TEXT UNIQUE,
                secondary_key TEXT UNIQUE,
                value BLOB
            )",
            table_name,
        );
        let mut stmt = conn.prepare(&statement_str)?;
        stmt.execute([])?;

        Ok(())
    }

    fn load_internal(
        conn: &'a Connection,
        primary_key_option: Option<&Self::Key>,
        secondary_key_option: Option<&Self::SecondaryKey>,
        load_multiple: bool,
    ) -> Result<Vec<Self>, PersistenceError> {
        let mut statement_str = format!("SELECT value FROM {}", Self::DATA_TYPE);

        // We prepare the query here, so we can use it in the match arms below.
        // This is due to annoying lifetime issues.
        let finalize_query = |params: &[&dyn ToSql], mut final_statement: String| {
            if !load_multiple {
                final_statement.push_str(" LIMIT 1");
            }
            let mut stmt = match conn.prepare(&final_statement) {
                Ok(stmt) => stmt,
                Err(e) => match e {
                    rusqlite::Error::SqliteFailure(_, Some(ref error_string)) => {
                        let expected_error_string = format!("no such table: {}", Self::DATA_TYPE);
                        if error_string == &expected_error_string {
                            return Ok(vec![]);
                        } else {
                            return Err(e.into());
                        }
                    }
                    _ => return Err(e.into()),
                },
            };

            let rows = stmt.query_map(
                params,
                |row: &Row<'_>| -> Result<Vec<u8>, rusqlite::Error> {
                    // We only query the value column, so we can use 0 as index here.
                    let value_bytes: Vec<u8> = row.get(0)?;
                    Ok(value_bytes)
                },
            )?;
            let values = rows
                .map(|row| {
                    let value_bytes = row?;
                    let payload = serde_json::from_slice(&value_bytes)?;
                    let value = Self::from_connection_and_payload(conn, payload);
                    Ok(value)
                })
                .collect::<Result<Vec<_>, PersistenceError>>()?;

            Ok(values)
        };
        match (primary_key_option.as_ref(), secondary_key_option.as_ref()) {
            // Loads all values
            (None, None) => finalize_query(params![], statement_str),
            // Loads values by secondary key
            (None, Some(key)) => {
                statement_str.push_str(" WHERE secondary_key = ?");
                finalize_query(params![key.to_sql_key()], statement_str)
            }
            // Loads values by primary key
            (Some(key), None) => {
                statement_str.push_str(" WHERE primary_key = ?");
                finalize_query(params![key.to_sql_key()], statement_str)
            }
            // Loads values by primary and secondary key
            (Some(pk), Some(sk)) => {
                statement_str.push_str(" WHERE primary_key = ? AND secondary_key = ?");
                finalize_query(params![pk.to_sql_key(), sk.to_sql_key()], statement_str)
            }
        }
    }

    fn load_one(
        conn: &'a Connection,
        primary_key_option: Option<&Self::Key>,
        secondary_key_option: Option<&Self::SecondaryKey>,
    ) -> Result<Option<Self>, PersistenceError> {
        let mut values =
            Self::load_internal(conn, primary_key_option, secondary_key_option, false)?;
        Ok(values.pop())
    }

    fn load(
        conn: &'a Connection,
        primary_key_option: Option<&Self::Key>,
        secondary_key_option: Option<&Self::SecondaryKey>,
    ) -> Result<Vec<Self>, PersistenceError> {
        Self::load_internal(conn, primary_key_option, secondary_key_option, true)
    }

    fn load_all(conn: &'a Connection) -> Result<Vec<Self>, PersistenceError> {
        Self::load(conn, None, None)
    }

    /// Persists this value in the database. If a value already exists for one of
    /// the unique columns, it will replace that value with this one.
    fn persist(&self) -> Result<(), PersistenceError> {
        let serialized_payload = serde_json::to_vec(self.payload())?;
        let statement_str = format!(
            "INSERT OR REPLACE INTO {} (primary_key, secondary_key, value) VALUES (:key, :secondary_key, :value)",
            Self::DATA_TYPE
        );
        let mut stmt = match self.connection().prepare(&statement_str) {
            Ok(stmt) => stmt,
            // If the table does not exist, we create it and try again.
            Err(e) => match e {
                rusqlite::Error::SqliteFailure(_, Some(ref error_string)) => {
                    let expected_error_string = format!("no such table: {}", Self::DATA_TYPE);
                    if error_string == &expected_error_string {
                        Self::create_table(self.connection())?;
                    } else {
                        return Err(e.into());
                    }
                    self.connection().prepare(&statement_str)?
                }
                _ => return Err(e.into()),
            },
        };
        stmt.insert(
            named_params! {":key": self.key().to_sql_key(), ":secondary_key": self.secondary_key().to_sql_key(),":value": serialized_payload},
        )?;

        Ok(())
    }

    fn purge(&self) -> Result<(), PersistenceError> {
        let key = self.key();
        Self::purge_key(self.connection(), key)
    }

    fn purge_key(conn: &Connection, key: &Self::Key) -> Result<(), PersistenceError> {
        let statement_str = format!("DELETE FROM {} WHERE primary_key = (:key)", Self::DATA_TYPE);
        let mut stmt = conn.prepare(&statement_str)?;
        stmt.execute(named_params! {":key": key.to_sql_key()})?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}
