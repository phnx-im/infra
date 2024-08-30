// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mls_assist::openmls::group::GroupId;
use phnxbackend::ds::{
    group_state::EncryptedDsGroupState, DsStorageProvider, LoadState, GROUP_STATE_EXPIRATION_DAYS,
};
use phnxtypes::{
    crypto::ear::Ciphertext,
    identifiers::{Fqdn, QualifiedGroupId, SealedClientReference},
    time::TimeStamp,
};
use rusqlite::{params, Connection};
use sqlx::types::Uuid;
use thiserror::Error;
use tls_codec::DeserializeBytes;

pub struct SqliteDsStorage {
    pub(crate) connection: SqliteConnection,
    pub(crate) own_domain: Fqdn,
}

impl SqliteDsStorage {
    pub fn new(path: &str, own_domain: Fqdn) -> Result<Self, SqliteDsStorageError> {
        let connection = Arc::new(Mutex::new(Connection::open(path)?));
        let provider = Self {
            connection,
            own_domain,
        };
        provider.create_tables();
        Ok(provider)
    }

    pub fn new_in_memory(own_domain: Fqdn) -> Result<Self, SqliteDsStorageError> {
        let connection = Arc::new(Mutex::new(Connection::open_in_memory()?));
        let provider = Self {
            connection,
            own_domain,
        };
        provider.create_tables();
        Ok(provider)
    }

    fn create_tables(&self) {
        let connection = self.connection.lock().expect("Mutex poisoned");
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS encrypted_groups (
                    group_id UUID PRIMARY KEY,
                    ciphertext BLOB NOT NULL,
                    last_used TEXT NOT NULL,
                    deleted_queues BLOB NOT NULL
                )",
                [],
            )
            .expect("Failed to create table");
    }
}

#[derive(Debug, Error)]
pub enum SqliteDsStorageError {
    #[error(transparent)]
    DatabaseError(#[from] rusqlite::Error),
    #[error(transparent)]
    SerializationError(#[from] Cbor::Error),
    #[error("Invalid input")]
    InvalidInput,
    #[error("Mutex poisoned")]
    MutexError,
}

#[async_trait]
impl DsStorageProvider for SqliteDsStorage {
    type StorageError = SqliteDsStorageError;

    /// Loads the ds group state with the group ID.
    async fn load_group_state(&self, group_id: &GroupId) -> Result<LoadState, Self::StorageError> {
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(group_id.as_slice())
            .map_err(|_| SqliteDsStorageError::InvalidInput)?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);

        let connection = self
            .connection
            .lock()
            .map_err(|_| SqliteDsStorageError::MutexError)?;
        let (ciphertext_bytes, last_used, deleted_queues_bytes) = connection.query_row(
            "SELECT ciphertext, last_used, deleted_queues FROM encrypted_groups WHERE group_id = ?1",
            [group_uuid],
            |row| {
                let ciphertext_bytes: Vec<u8> = row.get(0)?;
                let last_used: DateTime<Utc> = row.get(1)?;
                let deleted_queues_bytes: Vec<u8> = row.get(2)?;

                Ok((ciphertext_bytes, last_used, deleted_queues_bytes))
            },
        ).map_err(|e| {
            tracing::warn!("Error loading group state: {:?}", e);
            e
        })?;

        // A group id is reserved if it is in the database but has an empty
        // ciphertext and deleted queues.
        // TODO: We need a canonical version to signal if a group id is
        // reserved. I think we can just pass an option in, but then we'd have
        // to remove the NON NULL requirement for ciphertexts. Also leave a TODO
        // that there is probably a better way of doing this.

        let result = match (ciphertext_bytes.as_slice(), deleted_queues_bytes.as_slice()) {
            ([], []) => Ok(LoadState::Reserved(TimeStamp::from(last_used))),
            (ciphertext_bytes, deleted_queues_bytes) => {
                let ciphertext: Ciphertext = Cbor::from_slice(ciphertext_bytes)?;
                let last_used = TimeStamp::from(last_used);
                if last_used.has_expired(GROUP_STATE_EXPIRATION_DAYS) {
                    Ok(LoadState::Expired)
                } else {
                    let deleted_queues: Vec<SealedClientReference> =
                        Cbor::from_slice(deleted_queues_bytes)?;
                    let encrypted_group_state = EncryptedDsGroupState {
                        ciphertext,
                        last_used,
                        deleted_queues,
                    };
                    Ok(LoadState::Success(encrypted_group_state))
                }
            }
        };
        result
    }

    /// Saves the ds group state with the group ID.
    async fn save_group_state(
        &self,
        group_id: &GroupId,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<(), Self::StorageError> {
        let qgid =
            QualifiedGroupId::tls_deserialize_exact_bytes(group_id.as_slice()).map_err(|e| {
                tracing::warn!("Error parsing group id: {:?}", e);
                SqliteDsStorageError::InvalidInput
            })?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);
        let last_used = Utc::now();
        let deleted_queues = Cbor::to_vec(&encrypted_group_state.deleted_queues)?;
        let ciphertext = Cbor::to_vec(&encrypted_group_state.ciphertext)?;

        let connection = self
            .connection
            .lock()
            .map_err(|_| SqliteDsStorageError::MutexError)?;

        // Insert the group state into the database.
        let mut statement = connection.prepare(
            "UPDATE encrypted_groups SET ciphertext = ?2, last_used = ?3, deleted_queues = ?4 WHERE group_id = ?1",
        )?;

        statement
            .execute(params![group_uuid, ciphertext, last_used, deleted_queues])
            .map_err(|e| {
                tracing::warn!("Error saving group state: {:?}", e);
                e
            })?;

        Ok(())
    }

    /// Reserves the ds group state slot with the given group ID.
    ///
    /// Returns false if the group ID is already taken and true otherwise.
    async fn reserve_group_id(&self, group_id: &GroupId) -> Result<bool, Self::StorageError> {
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(group_id.as_slice())
            .map_err(|_| SqliteDsStorageError::InvalidInput)?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);

        let connection = self
            .connection
            .lock()
            .map_err(|_| SqliteDsStorageError::MutexError)?;

        // Check in the DB if the group ID is already taken and if not, add it to the DB.
        let mut statement = connection.prepare(
            "SELECT ciphertext, last_used, deleted_queues FROM encrypted_groups WHERE group_id = ?1",
        )?;

        if statement.exists(params![group_uuid])? {
            return Ok(false);
        }
        let last_used = Utc::now();
        let deleted_queues = vec![];
        let ciphertext = vec![];

        // Insert the group state into the database.
        let mut statement = connection.prepare(
                "INSERT INTO encrypted_groups (group_id, ciphertext, last_used, deleted_queues) VALUES (?1, ?2, ?3, ?4)",
            )?;

        statement.execute(params![group_uuid, ciphertext, last_used, deleted_queues])?;

        return Ok(true);
    }

    /// Returns the domain of this DS.
    async fn own_domain(&self) -> Fqdn {
        self.own_domain.clone()
    }
}
