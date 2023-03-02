//! This module and its submodules contain structs and types to facilitate (EAR)
//! encryption of other structs on the backend. See the individual submodules
//! for details.

pub mod keys;
mod traits;

use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
pub use traits::{DecryptionError, EarEncryptable, EncryptionError};

use aes_gcm::Aes256Gcm;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// This type determines the AEAD scheme used for encryption at rest (EAR) by
/// the backend.
/// TODO: Replace with a key-committing scheme.
pub type Aead = Aes256Gcm;
/// Key size of the above AEAD scheme
const AEAD_KEY_SIZE: usize = 32;
const AEAD_NONCE_SIZE: usize = 12;

// Convenience struct that allows us to keep ciphertext and nonce together.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct Ciphertext {
    ciphertext: Vec<u8>,
    nonce: [u8; AEAD_NONCE_SIZE],
}
