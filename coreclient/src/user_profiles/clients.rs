// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides structs and functions to interact with clients of users
//! in the various groups an InfraClient is a member of.

use crate::{groups::ClientAuthInfo, utils::persistence::Storable};

impl Storable for ClientAuthInfo {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS client_credentials (
                rowid INTEGER PRIMARY KEY,
                user_name TEXT NOT NULL,
                FOREIGN KEY (user_name) REFERENCES users(user_name),
                client_id TEXT PRIMARY KEY,
                FOREIGN KEY (client_id) REFERENCES clients(client_id),
                client_credential BLOB NOT NULL,
                signature_ear_key BLOB NOT NULL,
            )";
}
