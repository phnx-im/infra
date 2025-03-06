// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt::Display, fs, path::Path};

use anyhow::{bail, Result};
use openmls::group::GroupId;
use phnxtypes::identifiers::AsClientId;
use rusqlite::{types::FromSql, ToSql};
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    migrate,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Database, Encode, Sqlite, SqlitePool, Type,
};
use tracing::error;

use crate::clients::store::ClientRecord;

pub(crate) const PHNX_DB_NAME: &str = "phnx.db";

/// Open a connection to the DB that contains records for all clients on this
/// device.
pub(crate) async fn open_phnx_db(client_db_path: &str) -> sqlx::Result<SqlitePool> {
    let db_url = format!("sqlite://{}/{}", client_db_path, PHNX_DB_NAME);
    let opts: SqliteConnectOptions = db_url.parse()?;
    let opts = opts.journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePool::connect_with(opts).await?;

    migrate!().run(&pool).await?;

    Ok(pool)
}

pub(crate) async fn open_db_in_memory() -> sqlx::Result<SqlitePool> {
    let conn_opts = SqliteConnectOptions::new()
        .journal_mode(SqliteJournalMode::Wal)
        .in_memory(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(conn_opts)
        .await?;
    migrate!().run(&pool).await?;
    Ok(pool)
}

/// Delete both the phnx.db and all client dbs from this device.
///
/// WARNING: This will delete all APP-data from this device! Also, this function
/// may panic.
pub async fn delete_databases(client_db_path: &str) -> Result<()> {
    let full_phnx_db_path = format!("{client_db_path}/{PHNX_DB_NAME}");
    if !Path::new(&full_phnx_db_path).exists() {
        bail!("phnx.db does not exist")
    }

    // First delete all client DBs.
    let phnx_db_connection = open_phnx_db(client_db_path).await?;
    if let Ok(client_records) = ClientRecord::load_all(&phnx_db_connection).await {
        for client_record in client_records {
            let client_db_name = client_db_name(&client_record.as_client_id);
            let client_db_path = format!("{client_db_path}/{client_db_name}");
            if let Err(error) = fs::remove_file(&client_db_path) {
                error!(%error, %client_db_path, "Failed to delete client DB")
            }
        }
    }

    // Finally, delete the phnx.db.
    fs::remove_file(full_phnx_db_path)?;
    Ok(())
}

pub async fn delete_client_database(db_path: &str, as_client_id: &AsClientId) -> Result<()> {
    // Delete the client DB
    let client_db_name = client_db_name(as_client_id);
    let client_db_path = format!("{db_path}/{client_db_name}");
    if let Err(error) = fs::remove_file(&client_db_path) {
        error!(%error, %client_db_path, "Failed to delete client DB")
    }

    // Delete the client record from the phnx DB
    let full_phnx_db_path = format!("{db_path}/{PHNX_DB_NAME}");
    if !Path::new(&full_phnx_db_path).exists() {
        bail!("phnx.db does not exist")
    }
    let phnx_db = open_phnx_db(&client_db_path).await?;
    ClientRecord::delete(&phnx_db, as_client_id).await?;

    Ok(())
}

fn client_db_name(as_client_id: &AsClientId) -> String {
    format!("{}.db", as_client_id)
}

pub async fn open_client_db(
    as_client_id: &AsClientId,
    client_db_path: &str,
) -> sqlx::Result<SqlitePool> {
    let client_db_name = client_db_name(as_client_id);
    let db_url = format!("sqlite://{}/{}", client_db_path, client_db_name);
    let opts: SqliteConnectOptions = db_url.parse()?;
    let opts = opts.journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePool::connect_with(opts).await?;

    migrate!().run(&pool).await?;

    Ok(pool)
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

/// Helper struct that allows us to use GroupId as sqlite input.
pub(crate) struct GroupIdRefWrapper<'a>(&'a GroupId);

impl<'a> From<&'a GroupId> for GroupIdRefWrapper<'a> {
    fn from(group_id: &'a GroupId) -> Self {
        Self(group_id)
    }
}

impl ToSql for GroupIdRefWrapper<'_> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.as_slice().to_sql()
    }
}

impl Display for GroupIdRefWrapper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.0.as_slice()))
    }
}

impl Type<Sqlite> for GroupIdRefWrapper<'_> {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for GroupIdRefWrapper<'q> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(&self.0.as_slice(), buf)
    }
}

pub(crate) struct GroupIdWrapper(pub(crate) GroupId);

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
