// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SQLx traits for signing keys.
//!
//! The `Type` derive macro couldn't be used because the sqlx(type_name) macro doesn't
//! support generic types.

use crate::crypto::secrets::SecretBytes;

use super::private_keys::{SigningKey, VerifyingKey};

impl<
    'q,
    KT: for<'encode> ::sqlx::encode::Encode<'encode, ::sqlx::Postgres>
        + ::sqlx::types::Type<::sqlx::Postgres>,
> ::sqlx::encode::Encode<'_, ::sqlx::Postgres> for SigningKey<KT>
{
    fn encode_by_ref(
        &self,
        buf: &mut ::sqlx::postgres::PgArgumentBuffer,
    ) -> ::std::result::Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
        let mut encoder = ::sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder.encode(&self.signing_key)?;
        encoder.encode(&self.verifying_key)?;
        encoder.finish();
        ::std::result::Result::Ok(::sqlx::encode::IsNull::No)
    }
}

impl<
    'r,
    KT: for<'decode> ::sqlx::decode::Decode<'decode, ::sqlx::Postgres>
        + ::sqlx::types::Type<::sqlx::Postgres>,
> ::sqlx::decode::Decode<'r, ::sqlx::Postgres> for SigningKey<KT>
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
        let signing_key = decoder.try_decode::<SecretBytes>()?;
        let verifying_key = decoder.try_decode::<VerifyingKey<KT>>()?;
        ::std::result::Result::Ok(SigningKey {
            signing_key,
            verifying_key,
        })
    }
}

impl<KT> ::sqlx::Type<::sqlx::Postgres> for SigningKey<KT> {
    fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::with_name("signing_key_data")
    }
}

impl<KT> ::sqlx::postgres::PgHasArrayType for SigningKey<KT> {
    fn array_type_info() -> ::sqlx::postgres::PgTypeInfo {
        ::sqlx::postgres::PgTypeInfo::array_of("signing_key_data")
    }
}
