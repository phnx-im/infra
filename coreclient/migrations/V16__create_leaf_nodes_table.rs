// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use barrel::{backend::Sqlite, types, Migration};

use crate::groups::openmls_provider::own_leaf_nodes::StorableLeafNode;
use crate::utils::persistence::Storable;

/// OpenMLS provider data
pub fn migration() -> String {
    <StorableLeafNode<u8> as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
