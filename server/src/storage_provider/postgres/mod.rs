// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::{Connection, Executor, PgConnection, PgPool};

use crate::configurations::DatabaseSettings;

pub mod auth_service;
pub mod ds;
pub mod qs;

#[cfg(test)]
pub mod tests;

async fn connect_to_database(settings: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    let mut connection =
        PgConnection::connect(&settings.connection_string_without_database()).await?;
    let db_exists = sqlx::query!(
        "select exists (
            SELECT datname FROM pg_catalog.pg_database WHERE datname = $1
        )",
        settings.database_name
    )
    .fetch_one(&mut connection)
    .await?;
    if !db_exists.exists.unwrap_or(false) {
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database_name).as_str())
            .await?;
    }
    // Migrate database
    let connection_pool = PgPool::connect(&settings.connection_string()).await?;
    sqlx::migrate!("./migrations").run(&connection_pool).await?;
    Ok(connection_pool)
}
