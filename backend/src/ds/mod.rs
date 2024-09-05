// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, sync::Arc};

use phnxtypes::{identifiers::Fqdn, time::Duration};
use sqlx::{Executor, PgPool};
use tokio::sync::Mutex;
use uuid::Uuid;

mod add_clients;
mod add_users;
mod delete_group;
pub mod group_state;
mod join_connection_group;
mod join_group;
pub mod process;
mod remove_clients;
mod remove_users;
mod resync_client;
mod self_remove_client;
mod update_client;

/// Number of days after its last use upon which a group state is considered
/// expired.
pub const GROUP_STATE_EXPIRATION: Duration = Duration::days(90);

pub struct Ds {
    own_domain: Fqdn,
    reserved_group_ids: Arc<Mutex<HashSet<Uuid>>>,
    db_pool: PgPool,
}

#[derive(Debug)]
pub(crate) struct ReservedGroupId(Uuid);

impl Ds {
    // Create a new Ds instance. This will also migrate the database to the
    // newest schema. `connection_string` is the connection string to the
    // database without the database name.
    pub async fn new(
        own_domain: Fqdn,
        connection_string: &str,
        db_name: &str,
    ) -> Result<Self, sqlx::Error> {
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

        // Migrate database
        let db_pool = PgPool::connect(&connection_string_with_db).await?;
        sqlx::migrate!("./migrations").run(&db_pool).await?;

        let ds = Self {
            own_domain,
            reserved_group_ids: Arc::new(Mutex::new(HashSet::new())),
            db_pool,
        };

        Ok(ds)
    }

    async fn reserve_group_id(&self, group_id: Uuid) -> bool {
        let mut reserved_group_ids = self.reserved_group_ids.lock().await;
        reserved_group_ids.insert(group_id)
    }

    async fn claim_reserved_group_id(&self, group_id: Uuid) -> Option<ReservedGroupId> {
        let mut reserved_group_ids = self.reserved_group_ids.lock().await;
        if reserved_group_ids.remove(&group_id) {
            Some(ReservedGroupId(group_id))
        } else {
            None
        }
    }

    fn own_domain(&self) -> &Fqdn {
        &self.own_domain
    }
}
