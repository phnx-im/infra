// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{codec::PhnxCodec, messages::client_as::ConnectionPackage};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, Encode, Postgres, Type, error::BoxDynError};

mod persistence;

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

impl Type<Postgres> for StorableConnectionPackage {
    fn type_info() -> <Postgres as Database>::TypeInfo {
        <Vec<u8> as Type<Postgres>>::type_info()
    }
}

impl Decode<'_, Postgres> for StorableConnectionPackage {
    fn decode(value: <Postgres as Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Postgres>::decode(value)?;
        Ok(PhnxCodec::from_slice(bytes)?)
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

impl Type<Postgres> for StorableConnectionPackageRef<'_> {
    fn type_info() -> <Postgres as Database>::TypeInfo {
        <Vec<u8> as Type<Postgres>>::type_info()
    }
}

impl Encode<'_, Postgres> for StorableConnectionPackageRef<'_> {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as Database>::ArgumentBuffer<'_>,
    ) -> Result<sqlx::encode::IsNull, BoxDynError> {
        PhnxCodec::to_vec(self)?.encode(buf)
    }
}
