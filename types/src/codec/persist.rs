// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(feature = "sqlite")]
use rusqlite::types::{FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{encode::IsNull, error::BoxDynError, Database, Decode, Encode, Postgres, Sqlite, Type};

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

impl<T: BlobPersist, DB: Database> Type<DB> for BlobPersisting<'_, T>
where
    for<'a> &'a [u8]: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <&[u8] as Type<DB>>::type_info()
    }
}

impl<'q, T: BlobPersist> Encode<'q, Postgres> for BlobPersisting<'_, T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        PhnxCodec::V1.serialize_to_writer(&self.0, &mut **buf)?;
        Ok(IsNull::No)
    }
}

impl<'q, T: BlobPersist> Encode<'q, Sqlite> for BlobPersisting<'_, T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let bytes = PhnxCodec::V1.serialize(&self.0)?;
        println!("{}", hex::encode(&bytes));
        Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl<T: BlobPersist, DB: Database> Type<DB> for BlobPersisted<T>
where
    for<'a> &'a [u8]: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <&[u8] as Type<DB>>::type_info()
    }
}

impl<'r, T: BlobPersist, DB: Database> Decode<'r, DB> for BlobPersisted<T>
where
    for<'a> &'a [u8]: Decode<'a, DB>,
{
    fn decode(value: DB::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<DB>::decode(value)?;
        let value: T = PhnxCodec::from_slice(bytes)?;
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
        let value: T =
            PhnxCodec::from_slice(bytes).map_err(|error| FromSqlError::Other(Box::new(error)))?;
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sqlx::SqlitePool;

    use super::*;

    #[sqlx::test]
    async fn test_sqlite_blob_persist(pool: SqlitePool) -> sqlx::Result<()> {
        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct User {
            name: String,
            age: u32,
        }

        let ellie = User {
            name: "Ellie".to_owned(),
            age: 16,
        };
        let joel = User {
            name: "Joel".to_owned(),
            age: 50,
        };

        impl BlobPersist for User {}

        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO users (id, data) VALUES (1, ?)")
            .bind(ellie.persist())
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO users (id, data) VALUES (2, ?)")
            .bind(joel.persist())
            .execute(&pool)
            .await?;

        let users: Vec<BlobPersisted<User>> =
            sqlx::query_scalar("SELECT data FROM users ORDER BY id ASC")
                .fetch_all(&pool)
                .await?;
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].0, ellie);
        assert_eq!(users[1].0, joel);

        Ok(())
    }
}
