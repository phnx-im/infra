// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ops::Deref, path::Path};

use anyhow::{bail, Result};
use phnxtypes::identifiers::AsClientId;
use rusqlite::{named_params, params, Connection, Row, ToSql};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::clients::store::ClientRecord;

pub(crate) const PHNX_DB_NAME: &str = "phnx.db";

/// Open a connection to the DB that contains records for all clients on this
/// device.
pub(crate) fn open_phnx_db(client_db_path: &str) -> Result<Connection, PersistenceError> {
    let db_name = format!("{}/{}", client_db_path, PHNX_DB_NAME);
    let db_existed = Path::new(&db_name).exists();
    let conn = Connection::open(db_name)?;
    // Create a table for the client records if the db was newly created.
    if !db_existed {
        ClientRecord::create_table(&conn)?;
    }
    Ok(conn)
}

/// Delete both the phnx.db and all client dbs from this device.
///
/// WARNING: This will delete all APP-data from this device! Also, this function
/// may panic.
pub fn delete_databases(client_db_path: &str) -> Result<()> {
    use std::fs;

    let full_phnx_db_path = format!("{}/{}", client_db_path, PHNX_DB_NAME);
    if !Path::new(&full_phnx_db_path).exists() {
        bail!("phnx.db does not exist")
    }

    // First delete all client DBs.
    let phnx_db_connection = open_phnx_db(client_db_path)?;
    if let Ok(client_records) =
        PersistableStruct::<ClientRecord>::load_all_unfiltered(&phnx_db_connection)
    {
        for client_record in client_records {
            let full_client_db_path = format!(
                "{}/{}",
                client_db_path,
                client_db_name(&client_record.as_client_id)
            );
            if let Err(e) = fs::remove_file(full_client_db_path) {
                log::error!("Failed to delete client DB: {}", e)
            }
        }
    }

    // Finally, delete the phnx.db.
    fs::remove_file(full_phnx_db_path)?;
    Ok(())
}

fn client_db_name(as_client_id: &AsClientId) -> String {
    format!("{}.db", as_client_id)
}

pub(crate) fn open_client_db(
    as_client_id: &AsClientId,
    client_db_path: &str,
) -> Result<Connection, PersistenceError> {
    let client_db_name = client_db_name(as_client_id);
    let full_db_path = format!("{}/{}", client_db_path, client_db_name);
    let conn = Connection::open(full_db_path)?;
    Ok(conn)
}

pub(crate) struct PersistableStruct<'a, T: Persistable> {
    pub(crate) connection: &'a Connection,
    pub(crate) payload: T,
}

impl<'a, T: Persistable> Deref for PersistableStruct<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<'a, T: Persistable> PersistableStruct<'a, T> {
    fn connection(&self) -> &Connection {
        self.connection
    }
    pub(crate) fn payload(&self) -> &T {
        &self.payload
    }
    pub(crate) fn from_connection_and_payload(conn: &'a Connection, payload: T) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }

    /// Load a single value from the database. Returns `None` if no value was
    /// found for the given primary and/or secondary key(s).
    ///
    /// Returns an error either if the underlying database query fails or if
    /// deserialization of the returned value fails.
    pub(crate) fn load_one(
        conn: &'a Connection,
        primary_key_option: Option<&T::Key>,
        secondary_key_option: Option<&T::SecondaryKey>,
    ) -> Result<Option<Self>, PersistenceError> {
        let mut values = load_internal(conn, primary_key_option, secondary_key_option, Some(1))?;
        Ok(values.pop())
    }

    /// Load all values from the database that match the given key value(s).
    ///
    /// Returns an error either if the underlying database query fails or if
    /// deserialization of the returned value fails.
    pub(crate) fn load_all(
        conn: &'a Connection,
        primary_key_option: Option<&T::Key>,
        secondary_key_option: Option<&T::SecondaryKey>,
    ) -> Result<Vec<Self>, PersistenceError> {
        load_internal(conn, primary_key_option, secondary_key_option, None)
    }

    /// Load all values of this data type from the database. This is an alias
    /// for `load_all` with `None` as primary and secondary key.
    ///
    /// Returns an error either if the underlying database query fails or if
    /// deserialization of the returned value fails.
    pub(crate) fn load_all_unfiltered(conn: &'a Connection) -> Result<Vec<Self>, PersistenceError> {
        Self::load_all(conn, None, None)
    }

    /// Persists this value in the database. If a value already exists for one
    /// of the unique columns, it will replace that value with this one. If the
    /// table for the data type of this value does not exist, it will be
    /// created.
    ///
    /// Returns an error either if the underlying database query fails or if the
    /// serialization of this value fails.
    pub(crate) fn persist(&self) -> Result<(), PersistenceError> {
        let mut statement_str = format!(
            "INSERT OR REPLACE INTO {} (primary_key, secondary_key",
            T::DATA_TYPE.to_sql_key()
        );
        for field in T::additional_fields() {
            statement_str.push_str(", ");
            statement_str.push_str(field.field_name);
        }
        statement_str.push_str(") VALUES (?1, ?2");
        for index in 0..T::additional_fields().len() {
            statement_str.push_str(", ?");
            statement_str.push_str((index + 3).to_string().as_str());
        }
        statement_str.push_str(")");
        let mut stmt = self.connection().prepare(&statement_str)?;
        let mut fields: Vec<Box<dyn ToSql>> = vec![
            Box::new(self.payload().key().to_sql_key()),
            Box::new(self.payload().secondary_key().to_sql_key()),
        ];
        fields.append(&mut self.payload().get_sql_values()?);
        let field_refs: Vec<&dyn ToSql> = fields.iter().map(|e| e.as_ref()).collect();
        stmt.insert(field_refs.as_slice())?;

        Ok(())
    }

    /// Purges this value from the database.
    ///
    /// Returns an error either if the underlying database query fails.
    pub(crate) fn purge(&self) -> Result<(), PersistenceError> {
        let key = self.key();
        Self::purge_key(self.connection(), key)
    }

    /// Purges a value of this data type and with the given key from the
    /// database.
    ///
    /// Returns an error either if the underlying database query fails.
    pub(crate) fn purge_key(conn: &Connection, key: &T::Key) -> Result<(), PersistenceError> {
        let statement_str = format!(
            "DELETE FROM {} WHERE primary_key = (:key)",
            T::DATA_TYPE.to_sql_key()
        );
        let mut stmt = conn.prepare(&statement_str)?;
        stmt.execute(named_params! {":key": key.to_sql_key()})?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum DataType {
    // Data types in client dbs
    KeyStoreValue,
    UserProfile,
    Contact,
    PartialContact,
    Conversation,
    MlsGroup,
    Message,
    UnsentMessage,
    AsCredential,
    AsIntermediateCredential,
    LeafKeys,
    QsVerifyingKey,
    QueueRatchet,
    SequenceNumber,
    ClientData,
    RandomnessSeed,
    // Data types in phnx.db
    ClientRecord,
}

impl SqlKey for DataType {
    fn to_sql_key(&self) -> String {
        format!("{:?}", self)
    }
}

impl SqlKey for Uuid {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl SqlKey for String {
    fn to_sql_key(&self) -> String {
        self.clone()
    }
}

pub(crate) trait SqlKey {
    fn to_sql_key(&self) -> String;
}

pub(crate) struct SqlFieldDefinition {
    pub(crate) field_name: &'static str,
    pub(crate) field_keywords: &'static str,
}

impl From<(&'static str, &'static str)> for SqlFieldDefinition {
    fn from((field_name, field_keywords): (&'static str, &'static str)) -> Self {
        Self {
            field_name,
            field_keywords,
        }
    }
}

/// Trait for types that can be persisted in a SQLite database.
///
/// Tables for all data types are created when a client is created. New
/// implementers MUST be added to the `DataType` enum and added to the
/// `create_tables` function in `coreclient/src/utils/persistence.rs`.
pub(crate) trait Persistable: Sized + Serialize + DeserializeOwned {
    type Key: SqlKey + std::fmt::Debug;
    type SecondaryKey: SqlKey + std::fmt::Debug;

    const DATA_TYPE: DataType;

    /// Returns fields that (in addition to the default fields are required for
    /// the table of this data, as well as any associated keywords. Default
    /// fields are
    /// - "rowid INTEGER PRIMARY KEY",
    /// - "primary_key TEXT UNIQUE",
    /// - "secondary_key TEXT".
    fn additional_fields() -> Vec<SqlFieldDefinition> {
        let value_field_definition = SqlFieldDefinition {
            field_name: "value",
            field_keywords: "BLOB",
        };
        vec![value_field_definition]
    }

    /// The length of the vector resulting from this function MUST match up with
    /// the number of additional fields.
    fn get_sql_values(&self) -> Result<Vec<Box<dyn ToSql>>, PersistenceError> {
        let value = serde_json::to_vec(self)?;
        Ok(vec![Box::new(value)])
    }

    fn key(&self) -> &Self::Key;
    fn secondary_key(&self) -> &Self::SecondaryKey;

    /// Construct an instance of `Self` from a row of the SQL table. Note that
    /// the first three rows (indices 0 to 2) are the rowid, primary key and
    /// secondary key.
    fn try_from_row(row: &Row) -> Result<Self, PersistenceError> {
        let value: Vec<u8> = row.get(3)?;
        let payload = serde_json::from_slice(&value)?;
        Ok(payload)
    }

    /// Helper function that creates a table for the given data type.
    fn create_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let table_name = Self::DATA_TYPE.to_sql_key();
        let mut statement_str = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                rowid INTEGER PRIMARY KEY,
                primary_key TEXT UNIQUE,
                secondary_key TEXT",
            table_name,
        );
        for field in Self::additional_fields() {
            statement_str.push_str(", ");
            let sql_field_statement = format!("{} {}", field.field_name, field.field_keywords);
            statement_str.push_str(&sql_field_statement);
        }
        statement_str.push_str(")");
        let mut stmt = conn.prepare(&statement_str)?;
        stmt.execute([])?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("Failed to convert value from row")]
    ConversionError(#[from] anyhow::Error),
}

/// Helper function to read one or more values from the database. If
/// `number_of_entries` is set, it will load at most that number of entries.
fn load_internal<'a, T: Persistable>(
    conn: &'a Connection,
    primary_key_option: Option<&T::Key>,
    secondary_key_option: Option<&T::SecondaryKey>,
    number_of_entries_option: Option<u32>,
) -> Result<Vec<PersistableStruct<'a, T>>, PersistenceError> {
    let mut statement_str = "SELECT rowid, primary_key, secondary_key".to_string();
    for field in T::additional_fields() {
        statement_str.push_str(", ");
        statement_str.push_str(field.field_name);
    }
    statement_str.push_str(" FROM ");
    statement_str.push_str(T::DATA_TYPE.to_sql_key().as_str());

    // We prepare the query here, so we can use it in the match arms below.
    // This is due to annoying lifetime issues.
    let finalize_query = |params: &[&dyn ToSql], mut final_statement: String| {
        if matches!(T::DATA_TYPE, DataType::Message) {
            // We want to load messages in reverse order, so the use of LIMIT
            // gives us the most recent messages. We reverse the order of the
            // messages at the end of the query.
            final_statement.push_str(" ORDER BY timestamp DESC");
        }
        if let Some(number_of_entries) = number_of_entries_option {
            let limit_str = format!(" LIMIT {}", number_of_entries);
            final_statement.push_str(&limit_str);
        }
        let mut stmt = conn.prepare(&final_statement)?;

        let payloads = stmt.query(params)?;
        let values = payloads
            .and_then(|row| {
                let payload = T::try_from_row(row)?;
                let value = PersistableStruct::from_connection_and_payload(conn, payload);
                Ok(value)
            })
            .collect::<Result<Vec<PersistableStruct<'_, T>>, PersistenceError>>()?;

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
