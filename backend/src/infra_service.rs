// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use phnxtypes::identifiers::Fqdn;
use sqlx::{Executor, PgPool};
use thiserror::Error;

use crate::{errors::StorageError, settings::DatabaseSettings};

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
        database_settings: &DatabaseSettings,
        domain: Fqdn,
    ) -> Result<Self, ServiceCreationError> {
        let connection =
            PgPool::connect(&database_settings.connection_string_without_database()).await?;

        let db_name = database_settings.name.as_str();
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

        tracing::info!("Successfully created database {}", db_name);

        let db_pool = PgPool::connect(&database_settings.connection_string()).await?;

        Self::new_from_pool(db_pool, domain).await
    }

    async fn new_from_pool(db_pool: PgPool, domain: Fqdn) -> Result<Self, ServiceCreationError> {
        tracing::info!("Running database migration");
        sqlx::migrate!("./migrations").run(&db_pool).await?;
        tracing::info!("Database migration successful");

        Self::initialize(db_pool, domain).await
    }

    async fn initialize(db_pool: PgPool, domain: Fqdn) -> Result<Self, ServiceCreationError>;
}
