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

use hpke::{DecryptionKey, EncryptionPublicKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{LibraryError, crypto::ear::EarEncryptable, messages::QueueMessage};

use self::{
    ear::{Ciphertext, EarDecryptable, keys::RatchetKey},
    errors::RandomnessError,
    hpke::{HpkeDecryptionKey, HpkeEncryptionKey},
    kdf::{KdfDerivable, keys::RatchetSecret},
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
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct RatchetEncryptionKey(EncryptionPublicKey);

impl RatchetEncryptionKey {
    #[cfg(any(test, feature = "test_utils"))]
    pub fn new_for_test(encryption_key: EncryptionPublicKey) -> Self {
        Self(encryption_key)
    }
}

#[derive(Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct RatchetDecryptionKey(DecryptionKey);

impl RatchetDecryptionKey {
    pub fn generate() -> Result<Self, RandomnessError> {
        Ok(Self(DecryptionKey::generate()?))
    }

    pub fn encryption_key(&self) -> RatchetEncryptionKey {
        RatchetEncryptionKey(self.0.public_key().clone())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct ConnectionEncryptionKey {
    encryption_key: EncryptionPublicKey,
}

impl AsRef<EncryptionPublicKey> for ConnectionEncryptionKey {
    fn as_ref(&self) -> &EncryptionPublicKey {
        &self.encryption_key
    }
}

impl HpkeEncryptionKey for ConnectionEncryptionKey {}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConnectionDecryptionKey {
    decryption_key: DecryptionKey,
}

impl AsRef<DecryptionKey> for ConnectionDecryptionKey {
    fn as_ref(&self) -> &DecryptionKey {
        &self.decryption_key
    }
}

impl HpkeDecryptionKey for ConnectionDecryptionKey {}

impl ConnectionDecryptionKey {
    pub fn generate() -> Result<Self, RandomnessError> {
        Ok(Self {
            decryption_key: DecryptionKey::generate()?,
        })
    }

    pub fn encryption_key(&self) -> ConnectionEncryptionKey {
        ConnectionEncryptionKey {
            encryption_key: self.decryption_key.public_key().clone(),
        }
    }
}
