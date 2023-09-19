// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_rust_crypto::RustCrypto;
use openmls_traits::key_store::MlsEntity;

use crate::utils::persistence::PersistenceError;

use super::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct PhnxOpenMlsProvider {
    as_client_id: AsClientId,
    // TODO: Instead of skipping, we probably want to store the randomness here.
    #[serde(skip)]
    crypto: RustCrypto,
}

impl PhnxOpenMlsProvider {
    pub(crate) fn new(as_client_id: AsClientId) -> Self {
        let provider = Self {
            as_client_id,
            crypto: RustCrypto::default(),
        };
        provider
    }
}

impl OpenMlsProvider for PhnxOpenMlsProvider {
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

struct KeyStoreValue<'a> {
    conn: &'a Connection,
    key: String,
    payload: Vec<u8>,
}

impl<'a> Persistable<'a> for KeyStoreValue<'a> {
    type Key = String;

    type SecondaryKey = String;

    type Payload = Vec<u8>;

    const DATA_TYPE: DataType = DataType::KeyStoreValue;

    fn key(&self) -> &Self::Key {
        &self.key
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.key
    }

    fn connection(&self) -> &Connection {
        self.conn
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            conn,
            key: String::new(),
            payload,
        }
    }
}

impl OpenMlsKeyStore for PhnxOpenMlsProvider {
    /// The error type returned by the [`OpenMlsKeyStore`].
    type Error = PhnxKeyStorError;

    /// Store a value `v` that implements the [`ToKeyStoreValue`] trait for
    /// serialization for ID `k`.
    ///
    /// Returns an error if storing fails.
    fn store<V: MlsEntity>(&self, k: &[u8], v: &V) -> Result<(), Self::Error> {
        let value = serde_json::to_vec(v).map_err(|e| PersistenceError::SerdeError(e))?;
        let db_path = db_path(&self.as_client_id);
        let connection = Connection::open(db_path).map_err(|e| PersistenceError::SqliteError(e))?;

        let key_store_value = KeyStoreValue {
            conn: &connection,
            key: hex::encode(k),
            payload: value,
        };
        key_store_value.persist()?;
        Ok(())
    }

    /// Read and return a value stored for ID `k` that implements the
    /// [`FromKeyStoreValue`] trait for deserialization.
    ///
    /// Returns [`None`] if no value is stored for `k` or reading fails.
    fn read<V: MlsEntity>(&self, k: &[u8]) -> Option<V> {
        let db_path = db_path(&self.as_client_id);
        let connection = Connection::open(db_path)
            .map_err(|e| PersistenceError::SqliteError(e))
            .ok()?;
        let key_str = hex::encode(k);
        let key_store_value = match KeyStoreValue::load_one(&connection, Some(&key_str), None) {
            Ok(key_store_value) => key_store_value,
            Err(_) => return None,
        };

        serde_json::from_slice(&key_store_value?.payload).ok()
    }

    /// Delete a value stored for ID `k`.
    ///
    /// Returns an error if storing fails.
    fn delete<V: MlsEntity>(&self, k: &[u8]) -> Result<(), Self::Error> {
        let db_path = db_path(&self.as_client_id);
        let connection = Connection::open(db_path).map_err(|e| PersistenceError::SqliteError(e))?;
        let key_str = hex::encode(k);
        KeyStoreValue::purge_key(&connection, &key_str)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub(crate) enum PhnxKeyStorError {
    #[error(transparent)]
    PersistenceError(#[from] PersistenceError),
}
