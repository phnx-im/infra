// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module and its submodules contain structs and types to facilitate (EAR)
//! encryption of other structs on the backend. See the individual submodules
//! for details.

pub mod keys;
mod traits;

use std::marker::PhantomData;

use sqlx::{Postgres, postgres::PgHasArrayType};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
pub use traits::{
    EarDecryptable, EarEncryptable, EarKey, GenericDeserializable, GenericSerializable,
};

use aes_gcm::Aes256Gcm;
use serde::{Deserialize, Serialize};

/// This type determines the AEAD scheme used for encryption at rest (EAR) by
/// the backend.
/// TODO: Replace with a key-committing scheme.
pub type Aead = Aes256Gcm;
/// Key size of the [`Aead`] scheme
pub const AEAD_KEY_SIZE: usize = 32;
const AEAD_NONCE_SIZE: usize = 12;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    sqlx::Type,
)]
#[sqlx(type_name = "aead_ciphertext")]
pub struct AeadCiphertext {
    ciphertext: Vec<u8>,
    nonce: [u8; AEAD_NONCE_SIZE],
}

// Convenience struct that allows us to keep ciphertext and nonce together.
#[derive(
    Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TlsSize, TlsSerialize, TlsDeserializeBytes,
)]
#[serde(transparent)]
pub struct Ciphertext<CT> {
    ct: AeadCiphertext,
    pd: PhantomData<CT>,
}

impl<CT> From<AeadCiphertext> for Ciphertext<CT> {
    fn from(aead_ciphertext: AeadCiphertext) -> Self {
        Self {
            ct: aead_ciphertext,
            pd: PhantomData,
        }
    }
}

impl<CT> From<Ciphertext<CT>> for AeadCiphertext {
    fn from(ciphertext: Ciphertext<CT>) -> Self {
        ciphertext.ct
    }
}

impl<CT> sqlx::Type<Postgres> for Ciphertext<CT> {
    fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
        <AeadCiphertext as sqlx::Type<Postgres>>::type_info()
    }
}

impl<CT> PgHasArrayType for Ciphertext<CT> {
    fn array_type_info() -> <Postgres as sqlx::Database>::TypeInfo {
        <AeadCiphertext as PgHasArrayType>::array_type_info()
    }
}

impl<CT> sqlx::Encode<'_, Postgres> for Ciphertext<CT> {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        sqlx::Encode::<Postgres>::encode(&self.ct, buf)
    }
}

impl<CT> sqlx::Decode<'_, Postgres> for Ciphertext<CT> {
    fn decode(
        value: <Postgres as sqlx::Database>::ValueRef<'_>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let aead_ciphertext: AeadCiphertext = sqlx::Decode::<Postgres>::decode(value)?;
        Ok(Self {
            ct: aead_ciphertext,
            pd: PhantomData,
        })
    }
}

impl AeadCiphertext {
    pub fn new(ciphertext: Vec<u8>, nonce: [u8; AEAD_NONCE_SIZE]) -> Self {
        Self { ciphertext, nonce }
    }

    pub fn into_parts(self) -> (Vec<u8>, [u8; AEAD_NONCE_SIZE]) {
        let Self { ciphertext, nonce } = self;
        (ciphertext, nonce)
    }
}

#[cfg(any(feature = "test_utils", test))]
impl<CT> Ciphertext<CT> {
    pub fn dummy() -> Self {
        AeadCiphertext::dummy().into()
    }

    pub fn random() -> Self {
        AeadCiphertext::random().into()
    }

    pub fn flip_bit(&mut self) {
        self.ct.flip_bit();
    }
}

#[cfg(any(feature = "test_utils", test))]
impl AeadCiphertext {
    pub fn dummy() -> Self {
        Self {
            ciphertext: vec![1u8; 32],
            nonce: [1u8; AEAD_NONCE_SIZE],
        }
    }

    pub fn random() -> Self {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        Self {
            ciphertext: rng.r#gen::<[u8; 32]>().into(),
            nonce: rng.r#gen::<[u8; AEAD_NONCE_SIZE]>(),
        }
    }

    fn flip_bit(&mut self) {
        let byte = self.ciphertext.pop().unwrap();
        self.ciphertext.push(byte ^ 1);
    }
}
