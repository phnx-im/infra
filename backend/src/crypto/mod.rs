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

use argon2::Argon2;
use chrono::{DateTime, Utc};
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
use opaque_ke::CipherSuite;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::ear::EarEncryptable, messages::QueueMessage, qs::SealedClientReference, LibraryError,
};

use self::{
    ear::{keys::RatchetKey, Ciphertext, EarDecryptable, EncryptionError},
    hpke::{HpkeDecryptionKey, HpkeEncryptionKey},
    kdf::{keys::RatchetSecret, KdfDerivable},
};

/// Default ciphersuite we use for OPAQUE
pub struct OpaqueCiphersuite;

impl CipherSuite for OpaqueCiphersuite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeGroup = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::key_exchange::tripledh::TripleDh;

    type Ksf = Argon2<'static>;
}

// The OPAQUE ciphersuite's Noe: The size of a serialized OPRF group element output from SerializeElement.
const OPAQUE_NOE: usize = 32;
// The OPAQUE ciphersuite's Nok: The size of an OPRF private key as output from DeriveKeyPair.
const OPAQUE_NOK: usize = 32;
// The OPAQUE ciphersuite's Nn: Nonce length.
const OPAQUE_NN: usize = 32;
// The OPAQUE ciphersuite's Nm: MAC length.
const OPAQUE_NM: usize = 64;
// The OPAQUE ciphersuite's Nh: Hash length.
const OPAQUE_NH: usize = 64;
// The OPAQUE ciphersuite's Npk: Public key length.
const OPAQUE_NPK: usize = 32;

// The size of an OPAQUE envelope (Nn + nM)
const OPAQUE_ENVELOPE_SIZE: usize = OPAQUE_NN + OPAQUE_NM;
const OPAQUE_CREDENTIAL_REQUEST_SIZE: usize = OPAQUE_NOE;
const OPAQUE_CREDENTIAL_RESPONSE_SIZE: usize =
    OPAQUE_NOE + OPAQUE_NN + OPAQUE_NPK + OPAQUE_NN + OPAQUE_NM;
const OPAQUE_AUTH_REQUEST_SIZE: usize = OPAQUE_NN + OPAQUE_NPK;
const OPAQUE_AUTH_RESPONSE_SIZE: usize = OPAQUE_NN + OPAQUE_NPK + OPAQUE_NM;

// The size of the blinded message, i.e. a serialized OPRF group element using the
// ciphersuite defined above.
pub(crate) const OPAQUE_REGISTRATION_REQUEST_SIZE: usize = OPAQUE_NOE;
// The size of the evaluated message, i.e. a serialized OPRF group element, plus that of the server public key using the
// ciphersuite defined above.
pub(crate) const OPAQUE_REGISTRATION_RESPONSE_SIZE: usize = OPAQUE_NOE + OPAQUE_NOK;
// The size of the client upload after successful registration: The client public key, as well as a masking key and an envelope.
pub(crate) const OPAQUE_REGISTRATION_RECORD_SIZE: usize =
    OPAQUE_NPK + OPAQUE_NH + OPAQUE_ENVELOPE_SIZE;

// The size of the KE1 struct
pub(crate) const OPAQUE_LOGIN_REQUEST_SIZE: usize =
    OPAQUE_CREDENTIAL_REQUEST_SIZE + OPAQUE_AUTH_REQUEST_SIZE;
// The size of the KE2 struct
pub(crate) const OPAQUE_LOGIN_RESPONSE_SIZE: usize =
    OPAQUE_CREDENTIAL_RESPONSE_SIZE + OPAQUE_AUTH_RESPONSE_SIZE;
// The size of the KE3 struct
pub(crate) const OPAQUE_LOGIN_FINISH_SIZE: usize = OPAQUE_NM;

/// This type determines the hash function used by the backend.
pub type Hash = Sha256;

pub mod ear;
pub mod hpke;
pub mod kdf;
pub mod mac;
pub mod ratchet;
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

#[derive(
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    ToSchema,
    Debug,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
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

#[derive(Debug, Clone)]
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
            .derive_hpke_keypair(HPKE_CONFIG, &key_seed);
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
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    ToSchema,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
)]
pub struct RatchetEncryptionKey {
    encryption_key: EncryptionPublicKey,
}

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

#[derive(
    Debug, Clone, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserializeBytes, TlsSize,
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
