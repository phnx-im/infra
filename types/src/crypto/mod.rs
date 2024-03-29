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
use std::marker::PhantomData;

use mls_assist::{
    openmls::prelude::{OpenMlsProvider, OpenMlsRand},
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{
        crypto::OpenMlsCrypto,
        types::{
            HpkeAeadType, HpkeCiphertext, HpkeConfig, HpkeKdfType, HpkeKemType, HpkePrivateKey,
        },
    },
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{crypto::ear::EarEncryptable, messages::QueueMessage, LibraryError};

use self::{
    ear::{keys::RatchetKey, Ciphertext, EarDecryptable, EncryptionError},
    errors::RandomnessError,
    hpke::{HpkeDecryptionKey, HpkeEncryptionKey},
    kdf::{keys::RatchetSecret, KdfDerivable},
};

pub use opaque::OpaqueCiphersuite;

/// This type determines the hash function used by the backend.
pub type Hash = Sha256;

pub mod ear;
pub mod errors;
pub mod hpke;
pub mod kdf;
pub mod mac;
pub mod opaque;
pub mod ratchet;
pub mod secrets;
pub(super) mod serde_arrays;
pub mod signatures;

pub type RatchetKeyUpdate = Vec<u8>;

#[derive(
    Clone, PartialEq, Serialize, Deserialize, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct EncryptionPublicKey {
    public_key: Vec<u8>,
}

impl From<Vec<u8>> for EncryptionPublicKey {
    fn from(value: Vec<u8>) -> Self {
        Self { public_key: value }
    }
}

pub const HPKE_CONFIG: HpkeConfig = HpkeConfig(
    HpkeKemType::DhKem25519,
    HpkeKdfType::HkdfSha256,
    HpkeAeadType::AesGcm256,
);

impl EncryptionPublicKey {
    /// Encrypt the given plaintext using this key.
    pub(crate) fn encrypt(&self, info: &[u8], aad: &[u8], plain_txt: &[u8]) -> HpkeCiphertext {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .hpke_seal(HPKE_CONFIG, &self.public_key, info, aad, plain_txt)
            // TODO: get rid of unwrap
            .unwrap()
    }
}

#[derive(Error, Debug, Clone)]
pub enum DecryptionError {
    /// Error decrypting ciphertext.
    #[error("Error decrypting ciphertext.")]
    DecryptionError,
    /// Error deserializing payload.
    #[error("Error deserializing payload.")]
    DeserializationError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionPrivateKey {
    private_key: HpkePrivateKey,
    public_key: EncryptionPublicKey,
}

impl DecryptionPrivateKey {
    pub fn new(private_key: HpkePrivateKey, public_key: EncryptionPublicKey) -> Self {
        Self {
            private_key,
            public_key,
        }
    }

    pub fn decrypt(
        &self,
        info: &[u8],
        aad: &[u8],
        ct: &HpkeCiphertext,
    ) -> Result<Vec<u8>, DecryptionError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .hpke_open(HPKE_CONFIG, ct, &self.private_key, info, aad)
            .map_err(|_| DecryptionError::DecryptionError)
    }

    pub fn generate() -> Result<Self, RandomnessError> {
        let provider = OpenMlsRustCrypto::default();
        let key_seed = provider
            .rand()
            .random_array::<32>()
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        let keypair = provider
            .crypto()
            .derive_hpke_keypair(HPKE_CONFIG, &key_seed)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        Ok(Self {
            private_key: keypair.private,
            public_key: EncryptionPublicKey {
                public_key: keypair.public,
            },
        })
    }

    pub fn public_key(&self) -> &EncryptionPublicKey {
        &self.public_key
    }
}

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct RatchetEncryptionKey {
    encryption_key: EncryptionPublicKey,
}

#[derive(Serialize, Deserialize)]
pub struct RatchetDecryptionKey {
    decryption_key: DecryptionPrivateKey,
}

impl RatchetDecryptionKey {
    pub fn generate() -> Result<Self, RandomnessError> {
        Ok(Self {
            decryption_key: DecryptionPrivateKey::generate()?,
        })
    }

    pub fn encryption_key(&self) -> RatchetEncryptionKey {
        RatchetEncryptionKey {
            encryption_key: self.decryption_key.public_key.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ConnectionEncryptionKey {
    encryption_key: EncryptionPublicKey,
}

impl AsRef<EncryptionPublicKey> for ConnectionEncryptionKey {
    fn as_ref(&self) -> &EncryptionPublicKey {
        &self.encryption_key
    }
}

impl HpkeEncryptionKey for ConnectionEncryptionKey {}

#[derive(Serialize, Deserialize)]
pub struct ConnectionDecryptionKey {
    decryption_key: DecryptionPrivateKey,
}

impl AsRef<DecryptionPrivateKey> for ConnectionDecryptionKey {
    fn as_ref(&self) -> &DecryptionPrivateKey {
        &self.decryption_key
    }
}

impl HpkeDecryptionKey for ConnectionDecryptionKey {}

impl ConnectionDecryptionKey {
    pub fn generate() -> Result<Self, RandomnessError> {
        Ok(Self {
            decryption_key: DecryptionPrivateKey::generate()?,
        })
    }

    pub fn encryption_key(&self) -> ConnectionEncryptionKey {
        ConnectionEncryptionKey {
            encryption_key: self.decryption_key.public_key.clone(),
        }
    }
}
