// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_rust_crypto::RustCrypto;
use openmls_traits::key_store::MlsEntity;
use turbosql::{execute, select, Turbosql};

use super::*;

#[derive(Default)]
pub(crate) struct PhnxOpenMlsProvider {
    crypto: RustCrypto,
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
        };
        if let Ok(old_value) = select!(KeyStoreValue "WHERE key = " k) {
            // If it exists, delete it from the DB. (We could probably just
            // read out the rowid of the existing group and set it for the
            // new group, but this does the trick.)
            execute!("DELETE FROM keystorevalue WHERE rowid = " old_value.rowid.unwrap())?;
        }
        key_store_value.insert()?;
        Ok(())
    }

    /// Read and return a value stored for ID `k` that implements the
    /// [`FromKeyStoreValue`] trait for deserialization.
    ///
    /// Returns [`None`] if no value is stored for `k` or reading fails.
    fn read<V: MlsEntity>(&self, k: &[u8]) -> Option<V> {
        let key_store_value = select!(KeyStoreValue "WHERE key = " k).ok()?;
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
