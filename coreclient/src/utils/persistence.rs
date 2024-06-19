// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt::Display, path::Path};

use anyhow::{bail, Result};
use openmls::group::GroupId;
use phnxtypes::identifiers::AsClientId;
use rusqlite::{types::FromSql, Connection, ToSql};

use crate::clients::store::ClientRecord;

pub(crate) const PHNX_DB_NAME: &str = "phnx.db";

/// Open a connection to the DB that contains records for all clients on this
/// device.
pub(crate) fn open_phnx_db(client_db_path: &str) -> Result<Connection, rusqlite::Error> {
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
    if let Ok(client_records) = ClientRecord::load_all(&phnx_db_connection) {
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
) -> Result<Connection, rusqlite::Error> {
    let client_db_name = client_db_name(as_client_id);
    let full_db_path = format!("{}/{}", client_db_path, client_db_name);
    let conn = Connection::open(full_db_path)?;
    Ok(conn)
}

/// Helper function to read one or more values from the database. If
/// `number_of_entries` is set, it will load at most that number of entries.

pub(crate) trait Storable {
    const CREATE_TABLE_STATEMENT: &'static str;

    /// Helper function that creates a table for the given data type.
    fn create_table(conn: &rusqlite::Connection) -> anyhow::Result<(), rusqlite::Error> {
        let mut stmt = conn.prepare(Self::CREATE_TABLE_STATEMENT)?;
        stmt.execute([])?;

        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error>
    where
        Self: Sized;
}

pub(crate) trait Triggerable {
    const CREATE_TRIGGER_STATEMENTS: &'static [&'static str];

    /// Helper function that creates a trigger for the given data type.
    fn create_trigger(conn: &rusqlite::Connection) -> anyhow::Result<(), rusqlite::Error> {
        for statement in Self::CREATE_TRIGGER_STATEMENTS.iter() {
            let mut stmt = conn.prepare(statement)?;
            stmt.execute([])?;
        }

        Ok(())
    }
}

/// Helper struct that allows us to use GroupId as sqlite input.
pub(crate) struct GroupIdRefWrapper<'a>(&'a GroupId);

impl<'a> From<&'a GroupId> for GroupIdRefWrapper<'a> {
    fn from(group_id: &'a GroupId) -> Self {
        Self(group_id)
    }
}

impl<'a> ToSql for GroupIdRefWrapper<'a> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.as_slice().to_sql()
    }
}

impl Display for GroupIdRefWrapper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.0.as_slice()))
    }
}

pub(crate) struct GroupIdWrapper(GroupId);

impl From<GroupIdWrapper> for GroupId {
    fn from(group_id: GroupIdWrapper) -> Self {
        group_id.0
    }
}

impl FromSql for GroupIdWrapper {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let group_id = GroupId::from_slice(value.as_blob()?);
        Ok(GroupIdWrapper(group_id))
    }
}
