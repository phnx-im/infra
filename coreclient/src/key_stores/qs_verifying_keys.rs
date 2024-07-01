// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use phnxtypes::{crypto::signatures::keys::QsVerifyingKey, identifiers::Fqdn};
use rusqlite::{params, OptionalExtension};
use tokio::sync::Mutex;

use crate::utils::persistence::{SqliteConnection, Storable};

use super::*;

pub(crate) struct StorableQsVerifyingKey {
    domain: Fqdn,
    verifying_key: QsVerifyingKey,
}

impl Storable for StorableQsVerifyingKey {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS qs_verifying_keys (
            domain TEXT PRIMARY KEY,
            verifying_key BLOB NOT NULL
        );";

    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let domain = row.get(0)?;
        let verifying_key = row.get(1)?;
        Ok(StorableQsVerifyingKey {
            domain,
            verifying_key,
        })
    }
}

impl Deref for StorableQsVerifyingKey {
    type Target = QsVerifyingKey;

    fn deref(&self) -> &Self::Target {
        &self.verifying_key
    }
}

impl StorableQsVerifyingKey {
    fn load(connection: &Connection, domain: &Fqdn) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection
            .prepare("SELECT domain, verifying_key FROM qs_verifying_keys WHERE domain = ?")?;
        statement
            .query_row(params![domain], StorableQsVerifyingKey::from_row)
            .optional()
    }

    fn store(&self, connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "INSERT OR REPLACE INTO qs_verifying_keys (domain, verifying_key) VALUES (?, ?)",
            params![self.domain, self.verifying_key],
        )?;
        Ok(())
    }
}

impl StorableQsVerifyingKey {
    pub(crate) async fn get(
        connection_mutex: SqliteConnection,
        domain: &Fqdn,
        api_clients: &ApiClients,
    ) -> Result<StorableQsVerifyingKey> {
        let connection = connection_mutex.lock();
        if let Some(verifying_key) = Self::load(&connection, domain)? {
            Ok(verifying_key)
        } else {
            drop(connection);
            let verifying_key_response = api_clients.get(domain)?.qs_verifying_key().await?;
            let verifying_key = Self {
                domain: domain.clone(),
                verifying_key: verifying_key_response.verifying_key,
            };
            let connection = connection_mutex.lock();
            verifying_key.store(&connection)?;
            Ok(verifying_key)
        }
    }
}
