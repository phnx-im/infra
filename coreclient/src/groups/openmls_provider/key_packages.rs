// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{Entity, Key, CURRENT_VERSION};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityRefWrapper, EntityWrapper, KeyRefWrapper};

pub(crate) struct StorableKeyPackage<KeyPackage: Entity<CURRENT_VERSION>>(pub KeyPackage);

impl<KeyPackage: Entity<CURRENT_VERSION>> Storable for StorableKeyPackage<KeyPackage> {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS key_packages (
        key_package_ref BLOB PRIMARY KEY,
        key_package BLOB NOT NULL
    );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let EntityWrapper(key_package) = row.get(0)?;
        Ok(Self(key_package))
    }
}

impl<KeyPackage: Entity<CURRENT_VERSION>> StorableKeyPackage<KeyPackage> {
    pub(super) fn load<KeyPackageRef: Key<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        key_package_ref: &KeyPackageRef,
    ) -> Result<Option<KeyPackage>, rusqlite::Error> {
        connection
            .query_row(
                "SELECT key_package FROM key_packages WHERE key_package_ref = ?1",
                params![KeyRefWrapper(key_package_ref)],
                |row| Self::from_row(row).map(|x| x.0),
            )
            .optional()
    }
}

pub(super) struct StorableKeyPackageRef<'a, KeyPackage: Entity<CURRENT_VERSION>>(
    pub &'a KeyPackage,
);

impl<KeyPackage: Entity<CURRENT_VERSION>> StorableKeyPackageRef<'_, KeyPackage> {
    pub(super) fn store<KeyPackageRef: Key<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        key_package_ref: &KeyPackageRef,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT INTO key_packages (key_package_ref, key_package) VALUES (?1, ?2)",
            params![KeyRefWrapper(key_package_ref), EntityRefWrapper(self.0)],
        )?;
        Ok(())
    }
}

pub(super) struct StorableHashRef<'a, KeyPackageRef: Key<CURRENT_VERSION>>(pub &'a KeyPackageRef);

impl<KeyPackageRef: Key<CURRENT_VERSION>> StorableHashRef<'_, KeyPackageRef> {
    pub(super) fn delete_key_package(
        &self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM key_packages WHERE key_package_ref = ?1",
            params![KeyRefWrapper(self.0)],
        )?;
        Ok(())
    }
}
