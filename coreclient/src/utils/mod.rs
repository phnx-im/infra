// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use persistence::Storable;
use rusqlite::{types::FromSql, Connection, ToSql};

pub(crate) mod persistence;
pub(crate) mod versioning;

pub(super) struct SchemaVersion {
    version: u32,
}

impl ToSql for SchemaVersion {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        Ok(rusqlite::types::ToSqlOutput::from(self.version))
    }
}

impl FromSql for SchemaVersion {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        Ok(SchemaVersion {
            version: value.as_i64()? as u32,
        })
    }
}

/// The version number of the database schema. This should be incremented
/// whenever the schema changes in a way that is not backwards-compatible. If
/// this changes, the "DEFAULT" value in the schema_version table should be
/// updated to match.
const CODE_SCHEMA_VERSION: u32 = 1;

impl Storable for SchemaVersion {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL DEFAULT 1
        );";
}

impl SchemaVersion {
    fn db_schema_version(connection: &Connection) -> Result<SchemaVersion, rusqlite::Error> {
        connection.query_row("SELECT version FROM schema_version", [], |row| {
            let version = row.get(0)?;
            Ok(version)
        })
    }
}
