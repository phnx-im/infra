// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Misc. functions

pub fn delete_databases(client_db_path: String) -> anyhow::Result<()> {
    phnxcoreclient::delete_databases(client_db_path.as_str())
}
