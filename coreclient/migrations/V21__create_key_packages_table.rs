// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later


use crate::groups::openmls_provider::key_packages::StorableKeyPackage;
use crate::utils::persistence::Storable;

/// OpenMLS provider data
pub fn migration() -> String {
    <StorableKeyPackage<u8> as Storable>::CREATE_TABLE_STATEMENT.to_string()
}
