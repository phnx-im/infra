// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt::Display, fs, path::Path, time::Duration};

use crate::{
    clients::store::{ClientRecord, DatabaseEncryptionKey},
    utils::data_migrations,
};
use aircommon::identifiers::UserId;
use anyhow::{Result, bail};
use openmls::group::GroupId;
use sqlx::{
    Database, Encode, Sqlite, SqlitePool, Type,
    encode::IsNull,
    error::BoxDynError,
    migrate,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tracing::{error, info};

pub(crate) const AIR_DB_NAME: &str = "air.db";

/// Open a connection to the DB that contains records for all clients on this
/// device.
pub(crate) async fn open_air_db(db_path: &str) -> sqlx::Result<SqlitePool> {
    let db_url = format!("sqlite://{db_path}/{AIR_DB_NAME}");
    let opts: SqliteConnectOptions = db_url.parse()?;
    let opts = opts
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await?;

    // Delete the old migration table if it exists
    const FIRST_MIGRATION: i64 = 20250115104336;
    if let Ok(Some(_)) = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT 1 FROM _sqlx_migrations WHERE version = {FIRST_MIGRATION}"
    ))
    .fetch_optional(&pool)
    .await
    {
        // The database is based on old migration
        sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
            .execute(&pool)
            .await?;
    }

    migrate!("migrations/air").run(&pool).await?;

    Ok(pool)
}

pub(crate) async fn open_db_in_memory() -> sqlx::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .journal_mode(SqliteJournalMode::Wal)
        .in_memory(true);
    let pool = SqlitePoolOptions::new()
        // More than one connection in memory is not supported.
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        // We have only a single connection, so fail fast when there is a deadlock when acquiring a
        // connection.
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(opts)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

/// Delete both the air.db and all client dbs from this device.
///
/// If the air.db exists, but cannot be opened, only the air.db is deleted.
///
/// WARNING: This will delete all APP-data from this device!
pub async fn delete_databases(client_db_path: &str) -> Result<()> {
    let full_air_db_path = format!("{client_db_path}/{AIR_DB_NAME}");
    if !Path::new(&full_air_db_path).exists() {
        bail!("{full_air_db_path} does not exist")
    }

    // First try to delete all client DBs
    if let Err(error) = delete_client_databases(client_db_path).await {
        error!(%error, "Failed to delete client DBs")
    }

    // Finally, delete the air.db
    info!(path =% full_air_db_path, "removing AIR DB");
    fs::remove_file(full_air_db_path)?;

    Ok(())
}

async fn delete_client_databases(client_db_path: &str) -> anyhow::Result<()> {
    let air_db_connection = open_air_db(client_db_path).await?;
    if let Ok(client_records) = ClientRecord::load_all(&air_db_connection).await {
        for client_record in client_records {
            let client_db_name = client_db_name(&client_record.user_id);
            let client_db_path = format!("{client_db_path}/{client_db_name}");
            info!(path =% client_db_path, "removing client DB");
            if let Err(error) = fs::remove_file(&client_db_path) {
                error!(%error, %client_db_path, "Failed to delete client DB")
            }
        }
    }
    Ok(())
}

pub async fn delete_client_database(db_path: &str, user_id: &UserId) -> Result<()> {
    // Delete the client DB
    let client_db_name = client_db_name(user_id);
    let client_db_path = format!("{db_path}/{client_db_name}");
    if let Err(error) = fs::remove_file(&client_db_path) {
        error!(%error, %client_db_path, "Failed to delete client DB")
    }

    // Delete the client record from the air DB
    let full_air_db_path = format!("{db_path}/{AIR_DB_NAME}");
    if !Path::new(&full_air_db_path).exists() {
        bail!("air.db does not exist")
    }
    let air_db = open_air_db(db_path).await?;
    ClientRecord::delete(&air_db, user_id).await?;

    Ok(())
}

fn client_db_name(user_id: &UserId) -> String {
    format!("{}@{}.db", user_id.uuid(), user_id.domain())
}

/// Open a client database with the provided DEK.
pub async fn open_client_db(
    user_id: &UserId,
    client_db_path: &str,
    dek: &DatabaseEncryptionKey,
) -> anyhow::Result<SqlitePool> {
    let client_db_name = client_db_name(user_id);
    let db_url = format!("sqlite://{client_db_path}/{client_db_name}");
    let opts: SqliteConnectOptions = db_url
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse database URL: {}", e))?;
    let opts = opts
        .pragma("key", dek.to_hex_string())
        // Explicitly setting default parameters
        .pragma("cipher_page_size", "4096")
        // TODO: Given that we use a random key, we don't need this many rounds
        .pragma("kdf_iter", "256000")
        .pragma("cipher_hmac_algorithm", "HMAC_SHA512")
        .pragma("cipher_kdf_algorithm", "PBKDF2_HMAC_SHA512")
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::default()
        .connect_with(opts)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

    data_migrations::migrate(&pool).await?;
    migrate!().run(&pool).await?;

    Ok(pool)
}

/// Helper struct that allows us to use GroupId as sqlite input.
pub(crate) struct GroupIdRefWrapper<'a>(&'a GroupId);

impl<'a> From<&'a GroupId> for GroupIdRefWrapper<'a> {
    fn from(group_id: &'a GroupId) -> Self {
        Self(group_id)
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

#[cfg(test)]
mod tests {
    use std::{fs, str::FromStr};
    use tempfile::tempdir;
    use uuid::Uuid;

    use super::open_client_db;
    use aircommon::identifiers::{Fqdn, UserId};

    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    const MARKER: &str = "SECRET_MARKER_12345";

    /// Test that the encrypted database does not contain the plaintext marker
    #[tokio::test]
    async fn encrypted_db_marker() -> anyhow::Result<()> {
        use aircommon::crypto::ear::keys::DatabaseEncryptionKey;

        let dir = tempdir()?;
        let plain_path = dir.path().join("plain.db");

        // Generate a DEK for testing
        let dek = DatabaseEncryptionKey::random()?;

        // Encrypted DB via real code path
        let user_id = UserId::new(
            Uuid::from_str("00000000-0000-0000-0000-000000000001")?,
            Fqdn::from_str("example.test")?,
        );
        let pool = open_client_db(&user_id, dir.path().to_str().unwrap(), &dek).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS t (val TEXT)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO t (val) VALUES (?1)")
            .bind(MARKER)
            .execute(&pool)
            .await?;
        pool.close().await;

        // Discover the encrypted db file path that open_client_db created
        let mut enc_path: Option<std::path::PathBuf> = None;
        for entry in std::fs::read_dir(dir.path())? {
            let entry = entry?;
            if let Some(ext) = entry.path().extension()
                && ext == "db"
                && entry.file_name() != "plain.db"
            {
                enc_path = Some(entry.path());
                break;
            }
        }
        let enc_path = enc_path.expect("encrypted db file should exist");

        // Plaintext DB created manually
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", plain_path.display()))?
            .journal_mode(SqliteJournalMode::Wal)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::default().connect_with(opts).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS t (val TEXT)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO t (val) VALUES (?1)")
            .bind(MARKER)
            .execute(&pool)
            .await?;
        pool.close().await;

        // Read raw bytes
        let enc_bytes = fs::read(&enc_path)?;
        let plain_bytes = fs::read(&plain_path)?;

        // Plain should contain the marker in clear; encrypted should not
        assert!(
            !enc_bytes
                .windows(MARKER.len())
                .any(|w| w == MARKER.as_bytes()),
            "marker was found in encrypted db bytes"
        );
        assert!(
            plain_bytes
                .windows(MARKER.len())
                .any(|w| w == MARKER.as_bytes()),
            "marker not found in plaintext db bytes"
        );

        Ok(())
    }
}
