// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A module that provides various traits and structs that allow the use of
//! cryptographic primitives such as AEAD, MACs and KDFs.
//!
//! TODO: Once const-generics allows the use of enums, we could get rid of a
//! number of structs and boilerplate code.
//! TODO: A proper RNG provider for use with all crypto functions that require
//! randomness, i.e. mainly secret and nonce sampling.
#![allow(unused_variables)]
use argon2::Argon2;
use chrono::{DateTime, Utc};
use hpke::{Hpke, HpkePrivateKey, HpkePublicKey};
use hpke_rs_crypto::types::{AeadAlgorithm, KdfAlgorithm, KemAlgorithm};
use hpke_rs_rust_crypto::HpkeRustCrypto;
use opaque_ke::CipherSuite;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::ear::EarEncryptable,
    messages::{client_ds::QueueMessagePayload, QueueMessage},
    qs::SealedClientReference,
    LibraryError,
};

use self::{
    ear::{keys::RatchetKey, Ciphertext, EncryptionError},
    kdf::{keys::RatchetSecret, KdfDerivable},
};

/// Default ciphersuite we use for OPAQUE
pub struct OpaqueCipherSuite;

impl CipherSuite for OpaqueCipherSuite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeGroup = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::key_exchange::tripledh::TripleDh;

    type Ksf = Argon2<'static>;
}

// The size of a blinded message, i.e. a serialized OPRF group element using the
// ciphersuite defined above.
pub(crate) const OPAQUE_REGISTRATION_REQUEST_SIZE: usize = 32;

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
    pub deleted_queues: Vec<SealedClientReference>,
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

pub(crate) type RatchetKeyUpdate = Vec<u8>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QueueRatchet {
    sequence_number: u64,
    secret: RatchetSecret,
    key: RatchetKey,
}

// TODO: Implement the ratchet key.
impl QueueRatchet {
    /// Initialize a new ratchet key.
    pub fn random() -> Result<Self, RandomnessError> {
        let secret = RatchetSecret::random()?;
        let key = RatchetKey::derive(&secret, Vec::new())
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self {
            sequence_number: 0,
            secret,
            key,
        })
    }

    /// Encrypt the given payload.
    pub fn encrypt(
        &mut self,
        payload: QueueMessagePayload,
    ) -> Result<QueueMessage, EncryptionError> {
        // TODO: We want domain separation: FQDN, UserID & ClientID.
        let ciphertext = payload.encrypt(&self.key)?;

        let secret = RatchetSecret::derive(&self.secret, Vec::new())
            .map_err(|_| EncryptionError::LibraryError)?;
        let key =
            RatchetKey::derive(&secret, Vec::new()).map_err(|_| EncryptionError::LibraryError)?;

        self.secret = secret;
        self.key = key;
        self.sequence_number += 1;

        Ok(QueueMessage {
            sequence_number: self.sequence_number,
            ciphertext,
        })
    }

    /// Decrypt the given payload.
    pub fn decrypt(&self, queue_message: QueueMessage) -> Vec<u8> {
        todo!()
    }

    /// Sample some fresh entropy and inject it into the current key. Returns the entropy.
    pub fn update(&mut self) -> RatchetKeyUpdate {
        todo!()
    }

    /// Get the current sequence number
    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }
}

#[derive(Serialize, Deserialize, ToSchema, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct HpkeCiphertext {
    pub kem_output: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EncryptionPublicKey {
    public_key: HpkePublicKey,
}

impl From<Vec<u8>> for EncryptionPublicKey {
    fn from(value: Vec<u8>) -> Self {
        Self {
            public_key: value.into(),
        }
    }
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
