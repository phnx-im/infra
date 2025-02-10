// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "sqlite")]
use rusqlite::types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "sqlx")]
use sqlx::{encode::IsNull, error::BoxDynError, Database, Postgres};

use super::PhnxCodec;

/// A marker trait for types that can be persisted as a blob in the database.
pub trait BlobPersist: Serialize + DeserializeOwned {
    fn persist(&self) -> BlobPersisting<'_, Self> {
        BlobPersisting(self)
    }
}

impl<T: BlobPersist> BlobPersist for Vec<T> {}

/// A wrapper type for persisting `T: BlobPersist` as a blob in the database.
///
/// Because of Rust's orphan rules, we can't implement sql related traits for `T: BlobPersist`
/// directly.
#[derive(Debug)]
pub struct BlobPersisting<'a, T: BlobPersist>(pub &'a T);

/// A wrapper type for retrieving `T: BlobPersist` as a blob from the database.
///
/// Because of Rust's orphan rules, we can't implement sql related traits for `T: BlobPersist`
/// directly.
#[derive(Debug)]
pub struct BlobPersisted<T: BlobPersist>(pub T);

impl<'a, T: BlobPersist> From<&'a T> for BlobPersisting<'a, T> {
    fn from(value: &'a T) -> Self {
        Self(value)
    }
}

impl<T: BlobPersist> BlobPersisted<T> {
    /// Returns the inner value.
    ///
    /// Clones the data if it is not already owned.
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "sqlx")]
impl<T: BlobPersist> sqlx::Type<Postgres> for BlobPersisting<'_, T> {
    fn type_info() -> <Postgres as Database>::TypeInfo {
        <&[u8] as sqlx::Type<Postgres>>::type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'q, T: BlobPersist> sqlx::Encode<'q, Postgres> for BlobPersisting<'_, T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        PhnxCodec::V1.serialize_to_writer(&self.0, &mut **buf)?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlx")]
impl<T: BlobPersist> sqlx::Type<Postgres> for BlobPersisted<T> {
    fn type_info() -> <Postgres as Database>::TypeInfo {
        <&[u8] as sqlx::Type<Postgres>>::type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'r, T: BlobPersist> sqlx::Decode<'r, Postgres> for BlobPersisted<T> {
    fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = value.as_bytes()?;
        let value: T = PhnxCodec::V1.deserialize(bytes)?;
        Ok(Self(value))
    }
}

#[cfg(feature = "sqlite")]
impl<T: BlobPersist> rusqlite::ToSql for BlobPersisting<'_, T> {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let bytes = PhnxCodec::V1
            .serialize(&self.0)
            .map_err(rusqlite::Error::ToSqlConversionFailure)?;
        Ok(ToSqlOutput::Owned(bytes.into()))
    }
}

#[cfg(feature = "sqlite")]
impl<T: BlobPersist> rusqlite::types::FromSql for BlobPersisted<T> {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        let value: T = PhnxCodec::V1
            .deserialize(bytes)
            .map_err(FromSqlError::Other)?;
        Ok(Self(value))
    }
}
