// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module and its submodules contain structs and types to facilitate (EAR)
//! encryption of other structs on the backend. See the individual submodules
//! for details.

pub mod keys;
mod traits;

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
/// Key size of the above AEAD scheme
const AEAD_KEY_SIZE: usize = 32;
const AEAD_NONCE_SIZE: usize = 12;

// Convenience struct that allows us to keep ciphertext and nonce together.
#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct Ciphertext {
    ciphertext: Vec<u8>,
    nonce: [u8; AEAD_NONCE_SIZE],
}

impl Default for Ciphertext {
    fn default() -> Self {
        Self {
            ciphertext: vec![],
            nonce: [0u8; AEAD_NONCE_SIZE],
        }
    }
}

#[cfg(feature = "test_utils")]
impl Ciphertext {
    pub fn dummy() -> Self {
        Self {
            ciphertext: vec![1u8; 32],
            nonce: [1u8; AEAD_NONCE_SIZE],
        }
    }

    pub fn flip_bit(&mut self) {
        let byte = self.ciphertext.pop().unwrap();
        self.ciphertext.push(byte ^ 1);
    }
}

//#[cfg(feature = "sqlx")]
//mod sqlx {
//    use sqlx::{
//        encode::IsNull,
//        error::BoxDynError,
//        postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
//        Decode, Encode, Postgres,
//    };
//
//    impl sqlx::Type<Postgres> for super::Ciphertext {
//        fn type_info() -> PgTypeInfo {
//            <Vec<u8> as sqlx::Type<Postgres>>::type_info()
//        }
//    }
//
//    //impl Encode<'_, Postgres> for super::Ciphertext {
//    //    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
//    //        #[derive(sqlx::Encode)]
//    //        struct Encodable {
//    //            ciphertext: Vec<u8>,
//    //            nonce: [u8; super::AEAD_NONCE_SIZE],
//    //        };
//    //        Encodable {
//    //            ciphertext: self.ciphertext.clone(),
//    //            nonce: self.nonce,
//    //        }
//    //        .encode_by_ref(buf)
//    //        //let ctxt_is_null = self.ciphertext.as_slice().encode_by_ref(buf)?;
//    //        //let nonce_is_null = self.nonce.encode_by_ref(buf)?;
//    //        //if matches!(ctxt_is_null, IsNull::Yes) && matches!(nonce_is_null, IsNull::Yes) {
//    //        //    Ok(IsNull::Yes)
//    //        //} else {
//    //        //    Ok(IsNull::No)
//    //        //}
//    //    }
//    //}
//
//    //impl Decode<'_, Postgres> for super::Ciphertext {
//    //    fn decode(value: PgValueRef) -> Result<Self, BoxDynError> {
//    //        let (ciphertext, nonce) = <(Vec<u8>, [u8; super::AEAD_NONCE_SIZE])>::decode(value)?;
//    //        Ok(Self { ciphertext, nonce })
//    //    }
//    //}
//
//    impl PgHasArrayType for super::Ciphertext {
//        fn array_type_info() -> PgTypeInfo {
//            Vec::<u8>::array_type_info()
//        }
//    }
//}
