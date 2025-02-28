// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::store::StoreNotification;

pub fn migration() -> String {
    StoreNotification::CREATE_TABLE_STATEMENT.to_string()
}
