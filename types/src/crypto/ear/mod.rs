// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module and its submodules contain structs and types to facilitate (EAR)
//! encryption of other structs on the backend. See the individual submodules
//! for details.

pub mod keys;
mod traits;

use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize, VLBytes};
pub use traits::{
    EarDecryptable, EarEncryptable, EarKey, EncryptionError, GenericDeserializable,
    GenericSerializable,
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
pub struct Ciphertext {
    ciphertext: VLBytes,
    nonce: [u8; AEAD_NONCE_SIZE],
}

impl Default for Ciphertext {
    fn default() -> Self {
        Self {
            ciphertext: VLBytes::new(vec![]),
            nonce: [0u8; AEAD_NONCE_SIZE],
        }
    }
}

impl Ciphertext {
    pub fn dummy() -> Self {
        Self {
            ciphertext: VLBytes::new(vec![1u8; 32]),
            nonce: [1u8; AEAD_NONCE_SIZE],
        }
    }
}
