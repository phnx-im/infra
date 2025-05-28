// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::messages::client_as::ConnectionPackage;
use serde::{Deserialize, Serialize};

pub(crate) mod persistence;

#[derive(Deserialize)]
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

#[derive(Serialize)]
pub(in crate::auth_service) enum StorableConnectionPackageRef<'a> {
    CurrentVersion(&'a ConnectionPackage),
}

impl<'a> From<&'a ConnectionPackage> for StorableConnectionPackageRef<'a> {
    fn from(connection_package: &'a ConnectionPackage) -> Self {
        StorableConnectionPackageRef::CurrentVersion(connection_package)
    }
}
