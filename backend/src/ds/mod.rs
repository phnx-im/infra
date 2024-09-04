// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use migrator::Migrator;
use phnxtypes::{identifiers::Fqdn, time::Duration};
use sea_orm::{ConnectOptions, Database, DbConn, DbErr, TransactionTrait};
use sea_orm_migration::MigratorTrait;

mod add_clients;
mod add_users;
mod delete_group;
pub mod group_state;
mod join_connection_group;
mod join_group;
pub mod migrator;
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
    db_connection: DbConn,
}

impl Ds {
    pub async fn new(
        own_domain: Fqdn,
        connection_string: impl Into<String>,
    ) -> Result<Self, DbErr> {
        let opt = ConnectOptions::new(connection_string);
        // Configure things like Timeouts here...

        let db_connection = Database::connect(opt).await?;

        let ds = Self {
            own_domain,
            db_connection,
        };

        ds.migrate().await?;

        Ok(ds)
    }

    #[cfg(any(test, feature = "test_utils"))]
    pub async fn new_ephemeral(own_domain: Fqdn) -> Result<Self, DbErr> {
        let connection_string = format!("sqlite::memory:");
        Self::new(own_domain, connection_string).await
    }

    async fn migrate(&self) -> Result<(), DbErr> {
        let transaction = self.db_connection.begin().await?;
        Migrator::up(&transaction, None).await?;
        transaction.commit().await?;
        Ok(())
    }

    fn own_domain(&self) -> &Fqdn {
        &self.own_domain
    }
}
