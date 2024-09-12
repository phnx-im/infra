// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later


use crate::groups::openmls_provider::group_data::StorableGroupData;
use crate::utils::persistence::Storable;

/// OpenMLS provider data
pub fn migration() -> String {
    <StorableGroupData<u8> as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
