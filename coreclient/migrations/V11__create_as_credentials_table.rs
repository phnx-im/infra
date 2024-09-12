// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use barrel::{backend::Sqlite, types, Migration};

use crate::key_stores::as_credentials::AsCredentials;
use crate::utils::persistence::Storable;

pub fn migration() -> String {
    <AsCredentials as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
