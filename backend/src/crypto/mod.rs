//! A module that provides various traits and structs that allow the use of
//! cryptographic primitives such as AEAD, MACs and KDFs.
//!
//! TODO: Once const-generics allows the use of enums, we could get rid of a
//! number of structs and boilerplate code.
//! TODO: A proper RNG provider for use with all crypto functions that require
//! randomness, i.e. mainly secret and nonce sampling.
#![allow(unused_variables)]
use chrono::{DateTime, Utc};
use hpke::{Hpke, HpkePrivateKey, HpkePublicKey};
use hpke_rs_crypto::types::{AeadAlgorithm, KdfAlgorithm, KemAlgorithm};
use hpke_rs_rust_crypto::HpkeRustCrypto;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use tracing::instrument;
use utoipa::ToSchema;

use crate::{messages::client_qs::EnqueuedMessage, qs::SealedQueueConfig, LibraryError};

use self::ear::Ciphertext;

/// This type determines the hash function used by the backend.
pub type Hash = Sha256;

pub mod ear;
pub mod kdf;
pub mod mac;
pub mod secrets;
pub(super) mod serde_arrays;
pub mod signatures;

#[derive(Debug)]
pub enum RandomnessError {
    InsufficientRandomness,
}

#[derive(Clone)]
pub struct EncryptedDsGroupState {
    pub ciphertext: Ciphertext,
    pub last_used: DateTime<Utc>,
    pub deleted_queues: Vec<SealedQueueConfig>,
}

impl From<Ciphertext> for EncryptedDsGroupState {
    fn from(ciphertext: Ciphertext) -> Self {
        Self {
            ciphertext,
            last_used: Utc::now(),
            deleted_queues: Vec::new(),
        }
    }
}

impl AsRef<Ciphertext> for EncryptedDsGroupState {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

impl EncryptedDsGroupState {
    /// Get a reference to the encrypted ds group state's last used.
    #[must_use]
    pub(crate) fn last_used(&self) -> &DateTime<Utc> {
        &self.last_used
    }
}

pub(crate) type RatchetKeyUpdate = Vec<u8>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct RatchetKey {
    sequence_number: u64,
    key: Vec<u8>,
}

impl RatchetKey {
    /// Initialize a new ratchet key.
    pub fn new(initial_key: Vec<u8>) -> Self {
        Self {
            sequence_number: 0,
            key: initial_key,
        }
    }

    /// Encrypt the given payload.
    pub fn encrypt(&self, payload: &[u8]) -> EnqueuedMessage {
        EnqueuedMessage {
            sequence_number: self.sequence_number,
            ciphertext: Vec::new(),
        }
    }

    /// Decrypt the given payload.
    pub fn decrypt(&self, enqueued_message: EnqueuedMessage) -> Vec<u8> {
        todo!()
    }

    /// Ratchet the current key forward and returns the old ratcheting key.
    #[instrument(level = "trace", skip_all)]
    pub fn ratchet_forward(&mut self) -> RatchetKey {
        self.sequence_number += 1;
        todo!()
    }

    /// Sample some fresh entropy and inject it into the current key. Returns the entropy.
    pub fn update(&mut self) -> RatchetKeyUpdate {
        todo!()
    }

    /// Get the current sequence number
    pub fn sequence_number(&self) -> u64 {
        todo!()
    }
}

pub struct HpkeCiphertext {
    pub kem_output: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EncryptionPublicKey {
    public_key: HpkePublicKey,
}

impl EncryptionPublicKey {
    /// Encrypt the given plaintext using this key.
    pub(crate) fn encrypt(
        &self,
        info: &[u8],
        aad: &[u8],
        plain_txt: &[u8],
    ) -> Result<HpkeCiphertext, LibraryError> {
        Hpke::<HpkeRustCrypto>::new(
            hpke::Mode::Base,
            KemAlgorithm::DhKem25519,
            KdfAlgorithm::HkdfSha256,
            AeadAlgorithm::Aes256Gcm,
        )
        .seal(&self.public_key, info, aad, plain_txt, None, None, None)
        .map_err(|_| LibraryError::unexpected_crypto_error("Error encrypting with HPKE."))
        .map(|(kem_output, ciphertext)| HpkeCiphertext {
            kem_output,
            ciphertext,
        })
    }
}

pub enum DecryptionError {
    DecryptionError,
}

#[derive(Debug)]
pub struct DecryptionPrivateKey {
    private_key: HpkePrivateKey,
}

impl DecryptionPrivateKey {
    pub(crate) fn decrypt(
        &self,
        enc: &[u8],
        info: &[u8],
        aad: &[u8],
        ct: &HpkeCiphertext,
    ) -> Result<Vec<u8>, DecryptionError> {
        Hpke::<HpkeRustCrypto>::new(
            hpke::Mode::Base,
            KemAlgorithm::DhKem25519,
            KdfAlgorithm::HkdfSha256,
            AeadAlgorithm::Aes256Gcm,
        )
        .open(
            &ct.kem_output,
            &self.private_key,
            info,
            aad,
            &ct.ciphertext,
            None,
            None,
            None,
        )
        .map_err(|_| DecryptionError::DecryptionError)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct RatchetPublicKey {
    encryption_key: EncryptionPublicKey,
}

impl RatchetPublicKey {
    /// Encrypt the given ratchet key update under this hpke public key.
    pub(crate) fn encrypt_ratchet_key_update(
        &self,
        ratchet_key_update: &RatchetKeyUpdate,
    ) -> Vec<u8> {
        todo!()
    }
}
