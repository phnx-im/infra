// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::messages::client_as::ConnectionPackage;
use serde::{Deserialize, Serialize};

mod persistence;

#[derive(Serialize, Deserialize)]
pub(in crate::auth_service) enum StorableConnectionPackage {
    CurrentVersion(ConnectionPackage),
}

impl From<StorableConnectionPackage> for ConnectionPackage {
    fn from(connection_package: StorableConnectionPackage) -> Self {
        match connection_package {
            StorableConnectionPackage::CurrentVersion(connection_package) => connection_package,
        }
    }
}

impl From<ConnectionPackage> for StorableConnectionPackage {
    fn from(connection_package: ConnectionPackage) -> Self {
        StorableConnectionPackage::CurrentVersion(connection_package)
    }
}
