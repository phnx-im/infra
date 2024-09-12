// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use barrel::{backend::Sqlite, types, Migration};

use crate::clients::store::UserCreationState;
use crate::utils::persistence::Storable;

pub fn migration() -> String {
    <UserCreationState as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
