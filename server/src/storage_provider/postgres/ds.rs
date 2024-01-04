// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use chrono::Utc;
use mls_assist::openmls::group::GroupId;
use phnxbackend::ds::{
    group_state::EncryptedDsGroupState, DsStorageProvider, LoadState, GROUP_STATE_EXPIRATION_DAYS,
};
use phnxtypes::{
    crypto::ear::Ciphertext,
    identifiers::{Fqdn, SealedClientReference, QualifiedGroupId},
    time::TimeStamp,
};
use sqlx::{types::Uuid, PgPool};
use thiserror::Error;
use tls_codec::DeserializeBytes;

use crate::configurations::DatabaseSettings;

use super::connect_to_database;

pub struct PostgresDsStorage {
    pool: PgPool,
    own_domain: Fqdn,
}

impl PostgresDsStorage {
    pub async fn new(settings: &DatabaseSettings, own_domain: Fqdn) -> Result<Self, PostgresStorageError> {

        let pool = connect_to_database(settings).await?;

        let provider = Self {
            pool,
            own_domain,
        };
        Ok(provider)
    }
}

#[derive(Debug, Error)]
pub enum PostgresStorageError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid input")]
    InvalidInput,
    #[error(transparent)]
    MigrationError(#[from] sqlx::migrate::MigrateError),
}

#[async_trait]
impl DsStorageProvider for PostgresDsStorage {
    type StorageError = PostgresStorageError;

    /// Loads the ds group state with the group ID.
    async fn load_group_state(&self, group_id: &GroupId) -> Result<LoadState, Self::StorageError> {
        let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice())
            .map_err(|_| PostgresStorageError::InvalidInput)?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);

        let record = sqlx::query!(
            "SELECT ciphertext, last_used, deleted_queues FROM encrypted_groups WHERE group_id = $1",
            group_uuid 
        ).fetch_one(&self.pool).await.map_err(|e| {
            tracing::warn!("Error loading group state: {:?}", e);
            e 
        }
        )?;
        // A group id is reserved if it is in the database but has an empty
        // ciphertext and deleted queues.
        let ciphertext_bytes: Vec<u8> = record.ciphertext;
        let deleted_queues_bytes: Vec<u8> = record.deleted_queues;
        // TODO: We need a canonical version to signal if a group id is
        // reserved. I think we can just pass an option in, but then we'd have
        // to remove the NON NULL requirement for ciphertexts. Also leave a TODO
        // that there is probably a better way of doing this.
        
        let result = match (ciphertext_bytes.as_slice(), deleted_queues_bytes.as_slice()) {
            ([], []) => Ok(LoadState::Reserved(TimeStamp::from(record.last_used))),
            (ciphertext_bytes, deleted_queues_bytes) => {
                let ciphertext: Ciphertext = serde_json::from_slice(ciphertext_bytes)?;
                let last_used = TimeStamp::from(record.last_used);
                if last_used.has_expired(GROUP_STATE_EXPIRATION_DAYS) {
                    Ok(LoadState::Expired)
                } else {
                    let deleted_queues: Vec<SealedClientReference> =
                        serde_json::from_slice(deleted_queues_bytes)?;
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
        let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice())
            .map_err(|e| { 
                tracing::warn!("Error parsing group id: {:?}", e);
                PostgresStorageError::InvalidInput 
            })?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);
        let last_used = Utc::now();
        let deleted_queues = serde_json::to_vec(&encrypted_group_state.deleted_queues)?;
        let ciphertext = serde_json::to_vec(&encrypted_group_state.ciphertext)?;

        // Insert the group state into the database.
        sqlx::query!(
            r#"UPDATE encrypted_groups SET ciphertext = $2, last_used = $3, deleted_queues = $4 WHERE group_id = $1"#,
            group_uuid,
            ciphertext,
            last_used,
            deleted_queues
        )
        .execute(&self.pool)
        .await.map_err(|e| { 
            tracing::warn!("Error saving group state: {:?}", e);
            e 
        }
        )?;

        Ok(())
    }

    /// Reserves the ds group state slot with the given group ID.
    ///
    /// Returns false if the group ID is already taken and true otherwise.
    async fn reserve_group_id(&self, group_id: &GroupId) -> Result<bool, Self::StorageError> {
        let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice())
            .map_err(|_| PostgresStorageError::InvalidInput)?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);

        // This can probably be optimized to do only one query.
        let existing_entry = sqlx::query!(
            "SELECT ciphertext, last_used, deleted_queues FROM encrypted_groups WHERE group_id = $1",
            group_uuid
        ).fetch_optional(&self.pool).await?;

        if matches!(existing_entry, Some(_)) {
            return Ok(false);
        }

        let last_used = Utc::now();
        let deleted_queues = vec![];
        let ciphertext = vec![];

        // Insert the group state into the database.
        sqlx::query!(
            "INSERT INTO encrypted_groups (group_id, ciphertext, last_used, deleted_queues) VALUES ($1, $2, $3, $4)",
            group_uuid,
            ciphertext,
            last_used,
            deleted_queues
        )
        .execute(&self.pool)
        .await?;

        return Ok(true);
    }

    /// Returns the domain of this DS.
    async fn own_domain(&self) -> Fqdn {
        self.own_domain.clone()
    }
}
