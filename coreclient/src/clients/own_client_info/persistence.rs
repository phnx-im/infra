// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::AsClientId;
use rusqlite::{params, Connection};
use sqlx::{query, SqlitePool};

use crate::utils::persistence::Storable;

use super::OwnClientInfo;

impl Storable for OwnClientInfo {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS own_client_info (
            server_url TEXT NOT NULL,
            qs_user_id BLOB NOT NULL,
            qs_client_id BLOB NOT NULL,
            as_user_name TEXT NOT NULL,
            as_client_uuid BLOB NOT NULL
        );";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let server_url = row.get(0)?;
        let qs_user_id = row.get(1)?;
        let qs_client_id = row.get(2)?;
        let as_user_name = row.get(3)?;
        let as_client_uuid = row.get(4)?;

        Ok(OwnClientInfo {
            server_url,
            qs_user_id,
            qs_client_id,
            as_client_id: AsClientId::new(as_user_name, as_client_uuid),
        })
    }
}

impl OwnClientInfo {
    pub(crate) fn store(&self, connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "INSERT INTO own_client_info (
                server_url,
                qs_user_id,
                qs_client_id,
                as_user_name,
                as_client_uuid
            ) VALUES (?, ?, ?, ?, ?)",
            params![
                self.server_url,
                self.qs_user_id,
                self.qs_client_id,
                self.as_client_id.user_name(),
                self.as_client_id.client_id(),
            ],
        )?;
        Ok(())
    }

    pub(crate) async fn store_2(&self, db: &SqlitePool) -> sqlx::Result<()> {
        let user_name = self.as_client_id.user_name();
        let client_id = self.as_client_id.client_id();
        query!(
            "INSERT INTO own_client_info
                (server_url, qs_user_id, qs_client_id, as_user_name, as_client_uuid)
                VALUES (?, ?, ?, ?, ?)",
            self.server_url,
            self.qs_user_id,
            self.qs_client_id,
            user_name,
            client_id,
        )
        .execute(db)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::identifiers::{QsClientId, QsUserId};
    use uuid::Uuid;

    use super::*;

    fn test_connection() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(OwnClientInfo::CREATE_TABLE_STATEMENT)
            .unwrap();
        connection
    }

    #[test]
    fn store() -> anyhow::Result<()> {
        let connection = test_connection();

        let own_client_info = OwnClientInfo {
            server_url: "https://localhost".to_string(),
            qs_user_id: QsUserId::random(),
            qs_client_id: QsClientId::random(&mut rand::thread_rng()),
            as_client_id: AsClientId::new("alice@localhost".parse()?, Uuid::new_v4()),
        };

        own_client_info.store(&connection)?;

        let loaded = connection
            .prepare("SELECT * FROM own_client_info")?
            .query_row([], OwnClientInfo::from_row)?;
        assert_eq!(loaded, own_client_info);

        Ok(())
    }
}
