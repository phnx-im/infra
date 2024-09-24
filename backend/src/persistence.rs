// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use phnxtypes::identifiers::Fqdn;
use sqlx::{Executor, PgPool};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxtypes::codec::Error),
}

impl From<sqlx::Error> for StorageError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.into())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for StorageError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Database(e.into())
    }
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Dynamic(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// General error while accessing the requested queue.
#[derive(Error, Debug)]
pub(super) enum QueueError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
}

impl From<sqlx::Error> for QueueError {
    fn from(e: sqlx::Error) -> Self {
        Self::Storage(e.into())
    }
}

impl From<phnxtypes::codec::Error> for QueueError {
    fn from(e: phnxtypes::codec::Error) -> Self {
        Self::Storage(e.into())
    }
}

#[derive(Debug, Error)]
pub enum ServiceCreationError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("Service initialization failed: {0}")]
    InitializationFailed(Box<dyn std::error::Error + Send + Sync>),
}

impl<T: Into<sqlx::Error>> From<T> for ServiceCreationError {
    fn from(e: T) -> Self {
        Self::Storage(StorageError::from(e.into()))
    }
}

#[async_trait]
pub trait InfraService: Sized {
    async fn new(
        connection_string: &str,
        db_name: &str,
        domain: Fqdn,
    ) -> Result<Self, ServiceCreationError> {
        let connection = PgPool::connect(connection_string).await?;

        let db_exists = sqlx::query!(
            "select exists (
            SELECT datname FROM pg_catalog.pg_database WHERE datname = $1
        )",
            db_name,
        )
        .fetch_one(&connection)
        .await?;

        if !db_exists.exists.unwrap_or(false) {
            connection
                .execute(format!(r#"CREATE DATABASE "{}";"#, db_name).as_str())
                .await?;
        }

        let connection_string_with_db = format!("{}/{}", connection_string, db_name);

        let db_pool = PgPool::connect(&connection_string_with_db).await?;

        Self::new_from_pool(db_pool, domain).await
    }

    async fn new_from_pool(db_pool: PgPool, domain: Fqdn) -> Result<Self, ServiceCreationError> {
        sqlx::migrate!("./migrations").run(&db_pool).await?;
        tracing::info!("Database migration successful");

        Self::initialize(db_pool, domain).await
    }

    async fn initialize(db_pool: PgPool, domain: Fqdn) -> Result<Self, ServiceCreationError>;
}
