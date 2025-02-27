// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{de::DeserializeOwned, Serialize};

/// A trait for types that can be persisted as a blob in the database.
///
/// # Examples
///
/// Example where `Persisting` and `Persisted` are trivially the same type as implementing
/// `BlobPersist`:
///
/// ```rust
/// use persist::{BlobPersist, BlobPersistRestore, BlobPersistStore};
/// use serde::{Deserialize, Serialize};
///
/// /// Data that is persisted as a blob.
/// #[derive(Serialize, Deserialize)]
/// struct Data {}
///
/// impl BlobPersistStore for &Data {}
/// impl BlobPersistRestore for Data {}
///
/// impl BlobPersist for Data {
///     type Persisting<'a> = &'a Self;
///     type Persisted = Self;
/// }
/// ```
///
/// Example with wrapper types:
///
/// ```rust
/// use persist::{BlobPersist, BlobPersistRestore, BlobPersistStore};
/// use serde::{Deserialize, Serialize};
///
/// /// Data that is persisted as a blob via two wrapper types: `OuterData` and `OuterDataRef`.
/// #[derive(Serialize, Deserialize)]
/// struct InnerData {}
///
/// #[derive(Deserialize)]
/// struct OuterData(InnerData);
///
/// impl BlobPersistRestore for OuterData {}
///
/// impl From<OuterData> for InnerData {
///     fn from(OuterData(value): OuterData) -> Self {
///         value
///     }
/// }
///
/// #[derive(Serialize)]
/// struct OuterDataRef<'a>(&'a InnerData);
///
/// impl BlobPersistStore for OuterDataRef<'_> {}
///
/// impl<'a> From<&'a InnerData> for OuterDataRef<'a> {
///     fn from(value: &'a InnerData) -> Self {
///         Self(value)
///     }
/// }
///
/// /// The implementation of `BlobPersist` for `InnerData` specifies that it is persisted via the
/// /// `OuterDataRef` wrapper type, and restored via the `OuterData` wrapper type.
/// ///
/// /// In particular, this makes it possible to persists the `InnerData` without owning it.
/// impl BlobPersist for InnerData {
///     type Persisting<'a> = OuterDataRef<'a>;
///     type Persisted = OuterData;
/// }
/// ```
pub trait BlobPersist {
    /// The type which is used to store the data in the database.
    ///
    /// Most of the time, this is just `&Self`, but it can be more complex if the data is wrapped.
    type Persisting<'a>: BlobPersistStore
    where
        Self: 'a;

    /// The type which is used to restore the data from the database.
    ///
    /// Most of the time, this is just `Self`, but it can be more complex if the data is wrapped.
    type Persisted: BlobPersistRestore;

    /// Returns the wrapped ref-owned data for storing it in the database.
    ///
    /// Note that there is no correspoing `restore` method, because
    /// `BlobPersisted<Self::Persisted>` is used in return position to restore the data via the
    /// correspoding sql query functions.
    fn persist(&self) -> BlobPersisting<Self::Persisting<'_>>
    where
        for<'a> Self::Persisting<'a>: From<&'a Self>,
    {
        BlobPersisting(Self::Persisting::from(self))
    }
}

/// Marker trait for types that can be used as `BlobPersist::Persisting`.
///
/// Used to make sure that only explicit and not any `T: Serialize` can be wrapped in
/// `BlobPersisting<T>`.
pub trait BlobPersistStore: Serialize {}

/// Marker trait for types that can be used as `BlobPersist::Persisted`.
///
/// Used to make sure that only explicit and not any `T: DeserializeOwned` can be wrapped in
/// `BlobPersisted<T>`.
pub trait BlobPersistRestore: DeserializeOwned + 'static {}

/// Wrapper type for storing a `T: BlobPersistStore` as a blob in the database.
///
/// Unfortunately, this type is needed due to the Rust's orphan rule. Otherwise, it is not possible
/// to implement `sqlx::Encode` for any `T: BlobPersistStore`.
#[derive(Debug)]
pub struct BlobPersisting<T: BlobPersistStore>(pub T);

/// Wrapper type for restoring a `T: BlobPersistRestore` as a blob from the database.
///
/// Unfortunately, this type is needed due to the Rust's orphan rule. Otherwise, it is not possible
/// to implement `sqlx::Decode` for any `T: BlobPersistRestore`.
#[derive(Debug)]
pub struct BlobPersisted<T: BlobPersistRestore>(pub T);

impl<T: BlobPersistRestore> BlobPersisted<T> {
    /// Returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Implements the `BlobPersist` trait for a type.
///
/// The types is persisted/restored without wrappers: `Persisting` is `&Self`, and `Persisted` is
/// `Self`.
#[macro_export]
macro_rules! mark_as_blob_persist {
    ($type:ty) => {
        impl $crate::codec::persist::BlobPersist for $type {
            type Persisting<'a> = &'a Self;
            type Persisted = Self;
        }

        impl $crate::codec::persist::BlobPersistStore for &$type {}
        impl $crate::codec::persist::BlobPersistRestore for $type {}
    };
}

/// Implement sqlx Encode/Decode for all `BlobPersist` implementing types
#[cfg(feature = "sqlx")]
mod sqlx_impl {
    use sqlx::{
        encode::IsNull, error::BoxDynError, postgres::PgArgumentBuffer, Database, Decode, Encode,
        Postgres, Type,
    };

    use crate::codec::PhnxCodec;

    use super::*;

    impl<Data: BlobPersistStore, DB: Database> Type<DB> for BlobPersisting<Data>
    where
        Vec<u8>: Type<DB>,
    {
        fn type_info() -> <DB as Database>::TypeInfo {
            Vec::<u8>::type_info()
        }
    }

    // Specialized to Posgres because serialization can be done directly into the buffer.
    impl<'q, Data: BlobPersistStore> Encode<'q, Postgres> for BlobPersisting<Data>
    where
        Vec<u8>: Encode<'q, Postgres>,
    {
        fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
            PhnxCodec::V1.serialize_to_writer(&self.0, &mut **buf)?;
            Ok(IsNull::No)
        }
    }

    impl<Data: BlobPersistRestore, DB: Database> Type<DB> for BlobPersisted<Data>
    where
        for<'a> &'a [u8]: Type<DB>,
    {
        fn type_info() -> <DB as Database>::TypeInfo {
            <&[u8] as Type<DB>>::type_info()
        }
    }

    impl<'r, DB: Database, Data: BlobPersistRestore> Decode<'r, DB> for BlobPersisted<Data>
    where
        for<'a> &'a [u8]: Decode<'a, DB>,
    {
        fn decode(value: DB::ValueRef<'r>) -> Result<Self, BoxDynError> {
            let bytes: &[u8] = Decode::<DB>::decode(value)?;
            let value: Data = PhnxCodec::from_slice(bytes)?;
            Ok(BlobPersisted(value))
        }
    }
}

/// Implement rusqlite ToSql/FromSql for all `BlobPersist` implementing types
#[cfg(feature = "sqlite")]
mod rusqlite_impl {
    use rusqlite::{
        types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
        ToSql,
    };

    use crate::codec::PhnxCodec;

    use super::*;

    impl<Data: BlobPersistStore> ToSql for BlobPersisting<Data> {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            let bytes = PhnxCodec::V1
                .serialize(&self.0)
                .map_err(rusqlite::Error::ToSqlConversionFailure)?;
            Ok(ToSqlOutput::Owned(bytes.into()))
        }
    }

    impl<Data: BlobPersistRestore> FromSql for BlobPersisted<Data> {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            let bytes = value.as_blob()?;
            let value: Data = PhnxCodec::from_slice(bytes)
                .map_err(|error| FromSqlError::Other(Box::new(error)))?;
            Ok(BlobPersisted(value))
        }
    }
}
