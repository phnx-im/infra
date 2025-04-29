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

use hpke::{DecryptionKey, EncryptionKey};
use kdf::keys::ConnectionKeyType;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{LibraryError, crypto::ear::EarEncryptable, messages::QueueMessage};

use self::{
    ear::{EarDecryptable, keys::RatchetKey},
    errors::RandomnessError,
    kdf::{KdfDerivable, keys::RatchetSecret},
};

pub use opaque::OpaqueCiphersuite;

/// This type determines the hash function used by the backend.
pub type Hash = Sha256;

pub mod ear;
pub mod errors;
pub mod hpke;
pub mod indexed_aead;
pub mod kdf;
pub mod opaque;
pub mod ratchet;
pub mod secrets;
pub(super) mod serde_arrays;
pub mod signatures;

/// Marker trait for keys that can be converted to and from raw bytes
pub trait RawKey {}

pub type RatchetKeyUpdate = Vec<u8>;

#[derive(Debug)]
pub struct RatchetKeyType;
pub type RatchetEncryptionKey = EncryptionKey<RatchetKeyType>;

impl RawKey for RatchetKeyType {}

pub type RatchetDecryptionKey = DecryptionKey<RatchetKeyType>;

pub type ConnectionEncryptionKey = EncryptionKey<ConnectionKeyType>;
pub type ConnectionDecryptionKey = DecryptionKey<ConnectionKeyType>;

#[cfg(test)]
mod test {
    use crate::codec::PhnxCodec;

    use super::*;

    #[test]
    fn encryption_key_serde_codec() {
        let key = RatchetEncryptionKey::new_for_test(vec![1, 2, 3]);
        insta::assert_binary_snapshot!(".cbor", PhnxCodec::to_vec(&key).unwrap());
    }

    #[test]
    fn encryption_key_serde_json() {
        let key = RatchetEncryptionKey::new_for_test(vec![1, 2, 3]);
        insta::assert_json_snapshot!(key);
    }
}
