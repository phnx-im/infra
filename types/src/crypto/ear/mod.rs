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
/// Key size of the [`Aead`] scheme
pub const AEAD_KEY_SIZE: usize = 32;
const AEAD_NONCE_SIZE: usize = 12;

// Convenience struct that allows us to keep ciphertext and nonce together.
#[derive(
    Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
#[cfg_attr(
    feature = "sqlx",
    derive(sqlx::Type),
    sqlx(type_name = "aead_ciphertext")
)]
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

#[cfg(any(feature = "test_utils", test))]
impl Ciphertext {
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
            ciphertext: rng.gen::<[u8; 32]>().into(),
            nonce: rng.gen::<[u8; AEAD_NONCE_SIZE]>(),
        }
    }

    pub fn flip_bit(&mut self) {
        let byte = self.ciphertext.pop().unwrap();
        self.ciphertext.push(byte ^ 1);
    }
}
