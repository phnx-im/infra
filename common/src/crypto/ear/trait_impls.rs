// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::marker::PhantomData;

use sqlx::{Postgres, Sqlite, postgres::PgHasArrayType};
use tls_codec::{DeserializeBytes, Serialize};

use super::{AeadCiphertext, Ciphertext};

impl<CT, DB> sqlx::Type<DB> for Ciphertext<CT>
where
    DB: sqlx::Database,
    AeadCiphertext: sqlx::Type<DB>,
{
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <AeadCiphertext as sqlx::Type<DB>>::type_info()
    }
}

impl<CT> PgHasArrayType for Ciphertext<CT> {
    fn array_type_info() -> <Postgres as sqlx::Database>::TypeInfo {
        <AeadCiphertext as PgHasArrayType>::array_type_info()
    }
}

impl<'q, CT, DB> sqlx::Encode<'q, DB> for Ciphertext<CT>
where
    DB: sqlx::Database,
    for<'a> AeadCiphertext: sqlx::Encode<'a, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        sqlx::Encode::<DB>::encode(&self.ct, buf)
    }
}

impl<CT, DB> sqlx::Decode<'_, DB> for Ciphertext<CT>
where
    DB: sqlx::Database,
    for<'a> AeadCiphertext: sqlx::Decode<'a, DB>,
{
    fn decode(
        value: <DB as sqlx::Database>::ValueRef<'_>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let aead_ciphertext: AeadCiphertext = sqlx::Decode::<DB>::decode(value)?;
        Ok(Self {
            ct: aead_ciphertext,
            pd: PhantomData,
        })
    }
}

impl<CT> tls_codec::Size for Ciphertext<CT> {
    fn tls_serialized_len(&self) -> usize {
        self.ct.tls_serialized_len()
    }
}

impl<CT> tls_codec::Serialize for Ciphertext<CT> {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.ct.tls_serialize(writer)
    }
}

impl<CT> tls_codec::DeserializeBytes for Ciphertext<CT> {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (aead_ciphertext, rest) = AeadCiphertext::tls_deserialize_bytes(bytes)?;
        Ok((
            Self {
                ct: aead_ciphertext,
                pd: PhantomData,
            },
            rest,
        ))
    }
}

impl<CT> Clone for Ciphertext<CT> {
    fn clone(&self) -> Self {
        Self {
            ct: self.ct.clone(),
            pd: PhantomData,
        }
    }
}
impl<CT> PartialEq for Ciphertext<CT> {
    fn eq(&self, other: &Self) -> bool {
        self.ct == other.ct
    }
}
impl<CT> Eq for Ciphertext<CT> {}

impl sqlx::Type<Sqlite> for AeadCiphertext {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<Sqlite>>::type_info()
    }
}

impl sqlx::Encode<'_, Sqlite> for AeadCiphertext {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let bytes = self.tls_serialize_detached().map_err(Box::new)?;
        sqlx::Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl sqlx::Decode<'_, Sqlite> for AeadCiphertext {
    fn decode(
        value: <Sqlite as sqlx::Database>::ValueRef<'_>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let bytes: &[u8] = sqlx::Decode::<Sqlite>::decode(value)?;
        Ok(Self::tls_deserialize_exact_bytes(bytes).map_err(Box::new)?)
    }
}
