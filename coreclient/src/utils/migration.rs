// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use migrations::EmbeddedMigration;
use refinery::Migration;
use tracing::{error, info};

refinery::embed_migrations!("migrations/refinery");

pub(crate) fn run_migrations(
    client_db_connection: &mut rusqlite::Connection,
) -> Result<(), refinery::Error> {
    for migration in migrations::runner().run_iter(client_db_connection) {
        post_process(migration?);
    }

    match migrations::runner().run(client_db_connection) {
        Ok(report) => {
            let num_migrations = report.applied_migrations().len();
            info!(num_migrations, "Applied migrations successfully",);
            Ok(())
        }
        Err(error) => {
            error!(%error, "Failed to apply migrations");
            Err(error)
        }
    }
}

fn post_process(migration: Migration) {
    // Perform post-processing for arbitrary migrations here.
    match migration.into() {
        EmbeddedMigration::CreateInitialTablesAndTriggers(_) => {}
        EmbeddedMigration::AddTimestampIndexes(_) => {}
    }
}
