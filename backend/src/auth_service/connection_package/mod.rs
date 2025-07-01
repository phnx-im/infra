// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::messages::connection_package::{ConnectionPackage, legacy::ConnectionPackageV1};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(crate) mod persistence;

#[derive(Deserialize)]
pub(in crate::auth_service) enum StorableConnectionPackage {
    // This is here so we successfully deserialize old connection packages.
    #[allow(dead_code)]
    #[serde(rename = "CurrentVersion")]
    V1(ConnectionPackageV1),
    V2(ConnectionPackage),
}

#[derive(Debug, Error)]
pub(in crate::auth_service) enum ConnectionPackageStorageError {
    #[error("Invalid connection package version: {}", actual)]
    InvalidVersion { actual: String },
}

impl From<ConnectionPackageStorageError> for sqlx::Error {
    fn from(error: ConnectionPackageStorageError) -> Self {
        sqlx::Error::Decode(error.into())
    }
}

impl TryFrom<StorableConnectionPackage> for ConnectionPackage {
    type Error = ConnectionPackageStorageError;

    fn try_from(connection_package: StorableConnectionPackage) -> Result<Self, Self::Error> {
        match connection_package {
            StorableConnectionPackage::V2(connection_package) => Ok(connection_package),
            StorableConnectionPackage::V1(_) => {
                Err(ConnectionPackageStorageError::InvalidVersion {
                    actual: "V1".to_string(),
                })
            }
        }
    }
}

impl From<ConnectionPackage> for StorableConnectionPackage {
    fn from(connection_package: ConnectionPackage) -> Self {
        StorableConnectionPackage::V2(connection_package)
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
