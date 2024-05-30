// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::persistence::Storable;
use rusqlite::{types::FromSql, Connection, OptionalExtension, ToSql};
use thiserror::Error;

pub(crate) struct SchemaVersion {
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

#[derive(Debug, Error)]
pub(crate) enum MigrationError {
    #[error("Error migrating the database: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("The code is older than the database schema version")]
    CodeTooOld,
}

impl SchemaVersion {
    fn db_schema_version(connection: &Connection) -> Result<SchemaVersion, rusqlite::Error> {
        let schema_version_option = connection
            .query_row("SELECT version FROM schema_version", [], |row| {
                let version = row.get(0)?;
                Ok(version)
            })
            .optional()?;
        match schema_version_option {
            Some(version) => Ok(version),
            None => {
                connection.execute("INSERT INTO schema_version DEFAULT VALUES", [])?;
                Ok(SchemaVersion { version: 1 })
            }
        }
    }

    pub(crate) fn migrate(connection: &mut Connection) -> Result<(), MigrationError> {
        // Migrate until we reach the current schema version
        loop {
            let db_schema_version = Self::db_schema_version(connection)?;

            // Perform the migration
            match db_schema_version.version {
                db_schema_version if db_schema_version > CODE_SCHEMA_VERSION => {
                    log::error!("The code is older than the database schema version. Code: {}, Database: {}", CODE_SCHEMA_VERSION, db_schema_version);
                    return Err(MigrationError::CodeTooOld);
                }
                db_schema_version if db_schema_version == CODE_SCHEMA_VERSION => {
                    log::info!("Database schema is up to date");
                    return Ok(());
                }
                db_schema_version => {
                    log::info!(
                        "Migrating database from version {} to {}",
                        db_schema_version,
                        db_schema_version + 1
                    );

                    let transaction = connection.transaction()?;
                    // Perform the migration by calling the appropriate
                    // migration function for the given `db_schema_version`

                    // ...

                    // After successfull migration update the schema version
                    transaction.execute(
                        "UPDATE schema_version SET version = ?",
                        [db_schema_version + 1],
                    )?;

                    // Commit the transaction
                    transaction.commit()?;
                    log::info!(
                        "Database migration to version {} complete",
                        CODE_SCHEMA_VERSION
                    )
                    // Now re-enter the loop to potentially perform the next
                    // migration step
                }
            }
        }
    }
}
