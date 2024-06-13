// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{HashReference, KeyPackage},
    CURRENT_VERSION,
};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{
    EntityRefWrapper, EntityWrapper, KeyRefWrapper, SqliteStorageProviderError,
};

impl<T: StorableKeyPackage> Storable for T {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS key_packages (
        key_package_ref BLOB PRIMARY KEY,
        key_package BLOB NOT NULL,
    )";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(key_package) = row.get(0)?;
        Ok(key_package)
    }
}

impl<T: KeyPackage<CURRENT_VERSION>> StorableKeyPackage for T {}

pub(crate) trait StorableKeyPackage: KeyPackage<CURRENT_VERSION> {
    fn store<KeyPackageRef: HashReference<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        key_package_ref: &KeyPackageRef,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "INSERT INTO key_packages (key_package_ref, key_package) VALUES (?1, ?2)",
            params![KeyRefWrapper(key_package_ref), EntityRefWrapper(self)],
        )?;
        Ok(())
    }

    fn load<KeyPackageRef: HashReference<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        key_package_ref: &KeyPackageRef,
    ) -> Result<Option<Self>, SqliteStorageProviderError> {
        let key_package = connection
            .query_row(
                "SELECT key_package FROM key_packages WHERE key_package_ref = ?1",
                params![KeyRefWrapper(key_package_ref)],
                Self::from_row,
            )
            .optional()?;
        Ok(key_package)
    }

    fn delete<KeyPackageRef: HashReference<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        key_package_ref: &KeyPackageRef,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM key_packages WHERE key_package_ref = ?1",
            params![KeyRefWrapper(key_package_ref)],
        )?;
        Ok(())
    }
}
