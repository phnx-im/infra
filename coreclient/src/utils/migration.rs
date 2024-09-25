// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use migrations::EmbeddedMigration;
use refinery::Migration;

refinery::embed_migrations!("migrations");

pub(crate) fn run_migrations(
    client_db_connection: &mut rusqlite::Connection,
) -> Result<(), refinery::Error> {
    for migration in migrations::runner().run_iter(client_db_connection) {
        post_process(migration?);
    }

    match migrations::runner().run(client_db_connection) {
        Ok(report) => {
            log::info!(
                "Applied migrations successfully. Migrations applied: {}",
                report.applied_migrations().len()
            );
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to apply migrations: {}", e);
            Err(e)
        }
    }
}

fn post_process(migration: Migration) {
    match migration.into() {
        EmbeddedMigration::CreateInitialTablesAndTriggers(_) => {
            // Perform post-processing for arbitrary migrations here.
        }
    }
}
