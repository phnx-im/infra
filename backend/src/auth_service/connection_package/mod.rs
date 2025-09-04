// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::messages::connection_package::ConnectionPackage;
use serde::{Deserialize, Serialize};

pub(crate) mod persistence;

#[derive(Deserialize)]
pub(in crate::auth_service) enum StorableConnectionPackage {
    V1(ConnectionPackage),
}

impl From<StorableConnectionPackage> for ConnectionPackage {
    fn from(connection_package: StorableConnectionPackage) -> Self {
        match connection_package {
            StorableConnectionPackage::V1(connection_package) => connection_package,
        }
    }
}

impl From<ConnectionPackage> for StorableConnectionPackage {
    fn from(connection_package: ConnectionPackage) -> Self {
        StorableConnectionPackage::V1(connection_package)
    }
}

#[derive(Serialize)]
pub(in crate::auth_service) enum StorableConnectionPackageRef<'a> {
    V2(&'a ConnectionPackage),
}

impl<'a> From<&'a ConnectionPackage> for StorableConnectionPackageRef<'a> {
    fn from(connection_package: &'a ConnectionPackage) -> Self {
        StorableConnectionPackageRef::V2(connection_package)
    }
}
