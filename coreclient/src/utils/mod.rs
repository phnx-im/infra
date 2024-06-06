// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use rusqlite::Connection;
use thiserror::Error;

use crate::clients::store::{create_all_tables, create_all_triggers};

pub(crate) mod persistence;

#[derive(Debug, Error)]
pub enum DatabaseSetupError {
    #[error("Error setting up tables in the database: {0}")]
    TableCreationError(#[from] rusqlite::Error),
}

/// Create all necessary tables and triggers in the DB (if they do not exist
/// yet) and migrate the database to the newest schema if necessary.
pub(crate) fn set_up_database(
    client_db_connection: &mut Connection,
) -> Result<(), DatabaseSetupError> {
    create_all_tables(&client_db_connection)?;
    create_all_triggers(&client_db_connection)?;
    Ok(())
}
