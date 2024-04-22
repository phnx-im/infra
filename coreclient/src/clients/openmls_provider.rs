// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_rust_crypto::RustCrypto;
use openmls_traits::key_store::MlsEntity;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::utils::persistence::{PersistableStruct, PersistenceError, SqlKey};

use super::*;

pub(crate) struct PhnxOpenMlsProvider<'a> {
    connection: &'a Connection,
    crypto: RustCrypto,
}

impl<'a> PhnxOpenMlsProvider<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self {
            connection,
            crypto: RustCrypto::default(),
        }
    }
}

impl<'a> OpenMlsProvider for PhnxOpenMlsProvider<'a> {
    type KeyStoreProvider = Self;
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

    /// Get the key store provider.
    fn key_store(&self) -> &Self::KeyStoreProvider {
        self
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct KeyStoreValue {
    key: String,
    payload: Vec<u8>,
}

type PersistableKeyStoreValue<'a> = PersistableStruct<'a, KeyStoreValue>;

impl Persistable for KeyStoreValue {
    type Key = String;

    type SecondaryKey = String;

    const DATA_TYPE: DataType = DataType::KeyStoreValue;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.key
    }
}

impl<'a> OpenMlsKeyStore for PhnxOpenMlsProvider<'a> {
    /// The error type returned by the [`OpenMlsKeyStore`].
    type Error = PersistenceError;

    /// Store a value `v` that implements the [`ToKeyStoreValue`] trait for
    /// serialization for ID `k`.
    ///
    /// Returns an error if storing fails.
    fn store<V: MlsEntity>(&self, k: &[u8], v: &V) -> Result<(), Self::Error> {
        let value = serde_json::to_vec(v).map_err(|e| PersistenceError::SerdeError(e))?;

        let key_store_value = KeyStoreValue {
            key: hex::encode(k),
            payload: value,
        };
        let pksv =
            PersistableKeyStoreValue::from_connection_and_payload(self.connection, key_store_value);
        pksv.persist()?;
        Ok(())
    }

    /// Read and return a value stored for ID `k` that implements the
    /// [`FromKeyStoreValue`] trait for deserialization.
    ///
    /// Returns [`None`] if no value is stored for `k` or reading fails.
    fn read<V: MlsEntity>(&self, k: &[u8]) -> Option<V> {
        let key_str = hex::encode(k);
        let key_store_value =
            match PersistableKeyStoreValue::load_one(self.connection, Some(&key_str), None) {
                Ok(key_store_value) => key_store_value,
                Err(_) => return None,
            };

        serde_json::from_slice(&key_store_value?.payload().payload).ok()
    }

    /// Delete a value stored for ID `k`.
    ///
    /// Returns an error if storing fails.
    fn delete<V: MlsEntity>(&self, k: &[u8]) -> Result<(), Self::Error> {
        let key_str = hex::encode(k);
        PersistableKeyStoreValue::purge_key(self.connection, &key_str)?;
        Ok(())
    }
}

impl<'a> OpenMlsRand for PhnxOpenMlsProvider<'a> {
    type Error = PhnxRandomnessError;

    fn random_array<const N: usize>(&self) -> std::result::Result<[u8; N], Self::Error> {
        // Load seed from DB.
        let seed = PersistableSeed::load_one(self.connection, Some(&0), None);
        let mut rng = if let Ok(Some(seed)) = seed {
            ChaCha20Rng::from_seed(seed.payload)
        } else {
            ChaCha20Rng::from_entropy()
        };
        let mut out = [0u8; N];
        rng.try_fill_bytes(&mut out)
            .map_err(|_| Self::Error::NotEnoughRandomness)?;
        // Write fresh seed to DB.
        PersistableSeed::from_rng(self.connection, &mut rng)?;
        Ok(out)
    }

    fn random_vec(&self, len: usize) -> std::result::Result<Vec<u8>, Self::Error> {
        // Load seed from DB.
        let seed = PersistableSeed::load_one(self.connection, Some(&0), None);
        let mut rng = if let Ok(Some(seed)) = seed {
            ChaCha20Rng::from_seed(seed.payload)
        } else {
            ChaCha20Rng::from_entropy()
        };
        let mut out = vec![0u8; len];
        rng.try_fill_bytes(&mut out)
            .map_err(|_| Self::Error::NotEnoughRandomness)?;
        // Write fresh seed to DB.
        PersistableSeed::from_rng(self.connection, &mut rng)?;
        Ok(out)
    }
}

#[derive(Debug, Error)]
pub(crate) enum PhnxRandomnessError {
    #[error(transparent)]
    StorageError(#[from] PersistenceError),
    #[error("Unable to collect enough randomness.")]
    NotEnoughRandomness,
}

pub(super) type PersistableSeed<'a> = PersistableStruct<'a, [u8; 32]>;

impl<'a> PersistableSeed<'a> {
    /// Store a new random seed in the database.
    pub(super) fn new_random(conn: &'a Connection) -> Result<(), PhnxRandomnessError> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut payload = [0u8; 32];
        rng.try_fill_bytes(&mut payload)
            .map_err(|_| PhnxRandomnessError::NotEnoughRandomness)?;
        Ok(Self::from_connection_and_payload(conn, payload).persist()?)
    }

    /// Generate a new seed from the given RNG and store it in the database.
    pub(super) fn from_rng(
        conn: &'a Connection,
        rng: &mut ChaCha20Rng,
    ) -> Result<(), PhnxRandomnessError> {
        let mut payload = [0u8; 32];
        rng.try_fill_bytes(&mut payload)
            .map_err(|_| PhnxRandomnessError::NotEnoughRandomness)?;
        Ok(Self::from_connection_and_payload(conn, payload).persist()?)
    }
}

impl SqlKey for u64 {
    fn to_sql_key(&self) -> String {
        self.to_string()
    }
}

impl Persistable for [u8; 32] {
    type Key = u64;

    type SecondaryKey = u64;

    const DATA_TYPE: DataType = DataType::RandomnessSeed;

    fn key(&self) -> &Self::Key {
        &0
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &0
    }
}

#[test]
fn randomness() {
    use std::collections::HashSet;
    let connection = Connection::open_in_memory().unwrap();
    <[u8; 32] as Persistable>::create_table(&connection).unwrap();

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
