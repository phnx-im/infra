// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_rust_crypto::RustCrypto;
use openmls_traits::key_store::MlsEntity;
use turbosql::{execute, select, Turbosql};

use super::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct PhnxOpenMlsProvider {
    client_id_bytes: Vec<u8>,
    // TODO: Instead of skipping, we probably want to store the randomness here.
    #[serde(skip)]
    crypto: RustCrypto,
}

impl PhnxOpenMlsProvider {
    pub(crate) fn new(client_id: &AsClientId) -> Result<Self, tls_codec::Error> {
        let client_id_bytes = client_id.tls_serialize_detached()?;
        let provider = Self {
            client_id_bytes,
            crypto: RustCrypto::default(),
        };
        Ok(provider)
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

#[derive(Turbosql)]
struct KeyStoreValue {
    rowid: Option<i64>,
    client_id: Option<Vec<u8>>,
    key: Option<Vec<u8>>,
    value: Option<Vec<u8>>,
}

impl OpenMlsKeyStore for PhnxOpenMlsProvider {
    /// The error type returned by the [`OpenMlsKeyStore`].
    type Error = turbosql::Error;

    /// Store a value `v` that implements the [`ToKeyStoreValue`] trait for
    /// serialization for ID `k`.
    ///
    /// Returns an error if storing fails.
    fn store<V: MlsEntity>(&self, k: &[u8], v: &V) -> Result<(), Self::Error> {
        let value = serde_json::to_vec(v)?;

        let key_store_value = KeyStoreValue {
            rowid: None,
            key: Some(k.to_vec()),
            value: Some(value),
            client_id: Some(self.client_id_bytes.clone()),
        };
        if let Ok(old_value) =
            select!(KeyStoreValue "WHERE key = " k " AND client_id = " self.client_id_bytes)
        {
            // If it exists, delete it from the DB. (We could probably just
            // read out the rowid of the existing group and set it for the
            // new group, but this does the trick.)
            execute!("DELETE FROM keystorevalue WHERE rowid = " old_value.rowid.unwrap() " AND client_id = " self.client_id_bytes)?;
        }
        key_store_value.insert()?;
        Ok(())
    }

    /// Read and return a value stored for ID `k` that implements the
    /// [`FromKeyStoreValue`] trait for deserialization.
    ///
    /// Returns [`None`] if no value is stored for `k` or reading fails.
    fn read<V: MlsEntity>(&self, k: &[u8]) -> Option<V> {
        let key_store_value =
            select!(KeyStoreValue "WHERE key = " k " AND client_id = " self.client_id_bytes)
                .ok()?;
        serde_json::from_slice(&key_store_value.value?).ok()
    }

    /// Delete a value stored for ID `k`.
    ///
    /// Returns an error if storing fails.
    fn delete<V: MlsEntity>(&self, k: &[u8]) -> Result<(), Self::Error> {
        execute!("DELETE FROM keystorevalue WHERE key = " k)?;
        Ok(())
    }
}
