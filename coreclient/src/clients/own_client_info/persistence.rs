// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::AsClientId;
use rusqlite::{params, Connection};

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
            as_client_id: AsClientId::compose(as_user_name, as_client_uuid),
        })
    }
}

impl OwnClientInfo {
    pub(crate) fn store(&self, connection: &Connection) -> rusqlite::Result<()> {
        connection.execute(
            "INSERT INTO own_client_info (server_url, qs_user_id, qs_client_id, as_user_name, as_client_uuid) VALUES (?, ?, ?, ?, ?)",
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
}
