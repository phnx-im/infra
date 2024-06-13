// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_traits::storage::{
    traits::{
        SignatureKeyPair as SignatureKeyPairTrait, SignaturePublicKey as SignaturePublicKeyTrait,
    },
    CURRENT_VERSION,
};
use rusqlite::{params, OptionalExtension};

use crate::utils::persistence::Storable;

use super::storage_provider::{EntityWrapper, KeyRefWrapper, SqliteStorageProviderError};

pub(crate) struct StorableSignatureKeyPairs {
    signature_key_bytes: Vec<u8>,
}

impl Storable for StorableSignatureKeyPairs {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS signature_keys (
        public_key BLOB PRIMARY KEY,
        signature_key BLOB NOT NULL,
    )";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let signature_key_bytes = row.get(0)?;
        Ok(Self {
            signature_key_bytes,
        })
    }
}

impl StorableSignatureKeyPairs {
    pub(super) fn new<SignatureKeyPair: SignatureKeyPairTrait<CURRENT_VERSION>>(
        signature_key_pair: &SignatureKeyPair,
    ) -> Result<Self, SqliteStorageProviderError> {
        let signature_key_bytes = serde_json::to_vec(signature_key_pair)?;
        Ok(Self {
            signature_key_bytes,
        })
    }

    pub(super) fn store<SignaturePublicKey: SignaturePublicKeyTrait<CURRENT_VERSION>>(
        &self,
        connection: &rusqlite::Connection,
        public_key: &SignaturePublicKey,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "INSERT INTO signature_keys (public_key, signature_key) VALUES (?1, ?2)",
            params![KeyRefWrapper(public_key), self.signature_key_bytes],
        )?;
        Ok(())
    }

    pub(super) fn load<
        SignaturePublicKey: SignaturePublicKeyTrait<CURRENT_VERSION>,
        SignatureKeyPair: SignatureKeyPairTrait<CURRENT_VERSION>,
    >(
        connection: &rusqlite::Connection,
        public_key: &SignaturePublicKey,
    ) -> Result<Option<SignatureKeyPair>, SqliteStorageProviderError> {
        let signature_key = connection
            .query_row(
                "SELECT signature_key FROM signature_keys WHERE public_key = ?1",
                params![KeyRefWrapper(public_key)],
                |row| {
                    let EntityWrapper(signature_key) = row.get(0)?;
                    Ok(signature_key)
                },
            )
            .optional()?;
        Ok(signature_key)
    }

    pub(super) fn delete<SignaturePublicKey: SignaturePublicKeyTrait<CURRENT_VERSION>>(
        connection: &rusqlite::Connection,
        public_key: &SignaturePublicKey,
    ) -> Result<(), SqliteStorageProviderError> {
        connection.execute(
            "DELETE FROM signature_keys WHERE public_key = ?1",
            params![KeyRefWrapper(public_key)],
        )?;
        Ok(())
    }
}
