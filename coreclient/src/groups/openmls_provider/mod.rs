// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_rust_crypto::RustCrypto;
use openmls_traits::random::OpenMlsRand;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use storage_provider::SqliteStorageProvider;
use thiserror::Error;

use super::*;

pub(crate) mod encryption_key_pairs;
pub(crate) mod epoch_key_pairs;
pub(crate) mod group_data;
pub(crate) mod key_packages;
pub(crate) mod own_leaf_nodes;
pub(crate) mod proposals;
pub(crate) mod psks;
pub(crate) mod signature_key_pairs;
pub(super) mod storage_provider;

pub(crate) struct PhnxOpenMlsProvider<'a> {
    storage: SqliteStorageProvider<'a>,
    crypto: RustCrypto,
}

impl<'a> PhnxOpenMlsProvider<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self {
            storage: SqliteStorageProvider::new(connection),
            crypto: RustCrypto::default(),
        }
    }
}

impl<'a> OpenMlsProvider for PhnxOpenMlsProvider<'a> {
    type StorageProvider = SqliteStorageProvider<'a>;
    type CryptoProvider = RustCrypto;
    type RandProvider = RustCrypto;

    /// Get the crypto provider.
    fn crypto(&self) -> &Self::CryptoProvider {
        &self.crypto
    }

    /// Get the randomness provider.
    fn rand(&self) -> &Self::RandProvider {
        &self.crypto
    }

    fn storage(&self) -> &Self::StorageProvider {
        &self.storage
    }
}

impl OpenMlsRand for PhnxOpenMlsProvider<'_> {
    type Error = PhnxRandomnessError;

    fn random_array<const N: usize>(&self) -> std::result::Result<[u8; N], Self::Error> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut out = [0u8; N];
        rng.try_fill_bytes(&mut out)
            .map_err(|_| Self::Error::NotEnoughRandomness)?;
        Ok(out)
    }

    fn random_vec(&self, len: usize) -> std::result::Result<Vec<u8>, Self::Error> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut out = vec![0u8; len];
        rng.try_fill_bytes(&mut out)
            .map_err(|_| Self::Error::NotEnoughRandomness)?;
        Ok(out)
    }
}

#[derive(Debug, Error)]
pub(crate) enum PhnxRandomnessError {
    #[error(transparent)]
    StorageError(#[from] rusqlite::Error),
    #[error("Unable to collect enough randomness.")]
    NotEnoughRandomness,
}

#[test]
fn randomness() {
    use std::collections::HashSet;
    let connection = Connection::open_in_memory().unwrap();

    let provider = PhnxOpenMlsProvider::new(&connection);
    let random_vec_1 = provider.random_vec(32).unwrap();
    let random_vec_2 = provider.random_vec(32).unwrap();
    let provider = PhnxOpenMlsProvider::new(&connection);
    let random_vec_3 = provider.random_vec(32).unwrap();
    let random_vec_4 = provider.random_vec(32).unwrap();
    let set = [random_vec_1, random_vec_2, random_vec_3, random_vec_4]
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    assert_eq!(set.len(), 4);
}
