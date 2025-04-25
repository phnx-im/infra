// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::crypto::secrets::SecretBytes;

use super::{ClientIdKeyType, DecryptionKey, EncryptionKey};

impl<'q, KT, DB: ::sqlx::Database> ::sqlx::encode::Encode<'q, DB> for EncryptionKey<KT>
where
    Vec<u8>: ::sqlx::encode::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as ::sqlx::database::Database>::ArgumentBuffer<'q>,
    ) -> ::std::result::Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
        <Vec<u8> as ::sqlx::encode::Encode<'q, DB>>::encode_by_ref(&self.key, buf)
    }
}

impl<'r, KT, DB: ::sqlx::Database> ::sqlx::decode::Decode<'r, DB> for EncryptionKey<KT>
where
    Vec<u8>: ::sqlx::decode::Decode<'r, DB>,
{
    fn decode(
        value: <DB as ::sqlx::database::Database>::ValueRef<'r>,
    ) -> ::std::result::Result<
        Self,
        ::std::boxed::Box<
            dyn ::std::error::Error + 'static + ::std::marker::Send + ::std::marker::Sync,
        >,
    > {
        <Vec<u8> as ::sqlx::decode::Decode<'r, DB>>::decode(value).map(From::from)
    }
}

impl<KT, DB: ::sqlx::Database> ::sqlx::Type<DB> for EncryptionKey<KT>
where
    Vec<u8>: ::sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <Vec<u8> as ::sqlx::Type<DB>>::type_info()
    }
}

impl<KT> ::sqlx::postgres::PgHasArrayType for EncryptionKey<KT>
where
    Vec<u8>: ::sqlx::postgres::PgHasArrayType,
{
    fn array_type_info() -> ::sqlx::postgres::PgTypeInfo {
        <Vec<u8> as ::sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

impl<
    KT: for<'encode> ::sqlx::encode::Encode<'encode, ::sqlx::Postgres>
        + ::sqlx::types::Type<::sqlx::Postgres>,
> ::sqlx::encode::Encode<'_, ::sqlx::Postgres> for DecryptionKey<KT>
{
    fn encode_by_ref(
        &self,
        buf: &mut ::sqlx::postgres::PgArgumentBuffer,
    ) -> ::std::result::Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
        let mut encoder = ::sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.decryption_key)?;
        encoder.encode(&self.encryption_key)?;
        encoder.finish();
        ::std::result::Result::Ok(::sqlx::encode::IsNull::No)
    }
}

impl<
    'r,
    KT: for<'decode> ::sqlx::decode::Decode<'decode, ::sqlx::Postgres>
        + ::sqlx::types::Type<::sqlx::Postgres>,
> ::sqlx::decode::Decode<'r, ::sqlx::Postgres> for DecryptionKey<KT>
{
    fn decode(
        value: ::sqlx::postgres::PgValueRef<'r>,
    ) -> ::std::result::Result<
        Self,
        ::std::boxed::Box<
            dyn ::std::error::Error + 'static + ::std::marker::Send + ::std::marker::Sync,
        >,
    > {
        let mut decoder = ::sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let decryption_key = decoder.try_decode::<SecretBytes>()?;
        let encryption_key = decoder.try_decode::<EncryptionKey<KT>>()?;
        ::std::result::Result::Ok(DecryptionKey {
            decryption_key,
            encryption_key,
        })
    }
}

impl<KT> ::sqlx::Type<::sqlx::Postgres> for DecryptionKey<KT> {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("decryption_key_data")
    }
}

impl<KT> ::sqlx::postgres::PgHasArrayType for DecryptionKey<KT> {
    fn array_type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::array_of("decryption_key_data")
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for ClientIdKeyType {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        Ok(sqlx::encode::IsNull::No)
    }
}

impl sqlx::Decode<'_, sqlx::Postgres> for ClientIdKeyType {
    fn decode(
        value: <sqlx::Postgres as sqlx::Database>::ValueRef<'_>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        Ok(ClientIdKeyType)
    }
}

impl sqlx::Type<sqlx::Postgres> for ClientIdKeyType {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <bool as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}
