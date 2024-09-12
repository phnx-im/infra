// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::user_profiles::UserProfile;
use crate::utils::persistence::Storable;

pub fn migration() -> String {
    <UserProfile as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
