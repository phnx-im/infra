// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::{
    Database, Postgres, Type,
    decode::Decode,
    encode::{Encode, IsNull},
    error::BoxDynError,
    postgres::{
        PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef,
        types::{PgRecordDecoder, PgRecordEncoder},
    },
};

use crate::crypto::secrets::SecretBytes;

use super::{ClientIdKeyType, DecryptionKey, EncryptionKey};

impl<'q, KT, DB: Database> Encode<'q, DB> for EncryptionKey<KT>
where
    Vec<u8>: Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <Vec<u8> as Encode<'q, DB>>::encode_by_ref(&self.key, buf)
    }
}

impl<'r, KT, DB: Database> Decode<'r, DB> for EncryptionKey<KT>
where
    Vec<u8>: Decode<'r, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        <Vec<u8> as Decode<'r, DB>>::decode(value).map(From::from)
    }
}

impl<KT, DB: Database> Type<DB> for EncryptionKey<KT>
where
    Vec<u8>: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <Vec<u8> as Type<DB>>::type_info()
    }
}

impl<KT> PgHasArrayType for EncryptionKey<KT>
where
    Vec<u8>: PgHasArrayType,
{
    fn array_type_info() -> PgTypeInfo {
        <Vec<u8> as PgHasArrayType>::array_type_info()
    }
}

impl<KT: for<'encode> Encode<'encode, Postgres> + Type<Postgres>> Encode<'_, Postgres>
    for DecryptionKey<KT>
{
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let mut encoder = PgRecordEncoder::new(buf);
        encoder.encode(&self.decryption_key)?;
        encoder.encode(&self.encryption_key)?;
        encoder.finish();
        Result::Ok(IsNull::No)
    }
}

impl<'r, KT: for<'decode> Decode<'decode, Postgres> + Type<Postgres>> Decode<'r, Postgres>
    for DecryptionKey<KT>
{
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let mut decoder = PgRecordDecoder::new(value)?;
        let decryption_key = decoder.try_decode::<SecretBytes>()?;
        let encryption_key = decoder.try_decode::<EncryptionKey<KT>>()?;
        Result::Ok(DecryptionKey {
            decryption_key,
            encryption_key,
        })
    }
}

impl<KT> Type<Postgres> for DecryptionKey<KT> {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("decryption_key_data")
    }
}

impl<KT> PgHasArrayType for DecryptionKey<KT> {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::array_of("decryption_key_data")
    }
}

impl sqlx::Encode<'_, Postgres> for ClientIdKeyType {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as Database>::ArgumentBuffer<'_>,
    ) -> Result<IsNull, BoxDynError> {
        Ok(IsNull::No)
    }
}

impl sqlx::Decode<'_, Postgres> for ClientIdKeyType {
    fn decode(value: <Postgres as Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
        Ok(ClientIdKeyType)
    }
}

impl sqlx::Type<Postgres> for ClientIdKeyType {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <bool as sqlx::Type<Postgres>>::type_info()
    }
}
