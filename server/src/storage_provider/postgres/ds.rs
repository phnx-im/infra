// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use chrono::Utc;
use phnxbackend::ds::{
    GenericLoadState, GROUP_STATE_EXPIRATION_DAYS, DsGenericStorageProvider, DsStorageProvider,
};
use phnxtypes::{
    identifiers::{Fqdn},
    time::TimeStamp,
};
use sqlx::{types::Uuid, PgPool, PgConnection, Connection, Executor};
use thiserror::Error;

use crate::configurations::DatabaseSettings;

pub struct PostgresDsStorage {
    pool: PgPool,
    own_domain: Fqdn,
}

impl PostgresDsStorage {
    pub async fn new(settings: &DatabaseSettings, own_domain: Fqdn) -> Result<Self, PostgresStorageError> {
        // Create database
        let mut connection = PgConnection::connect(&settings.connection_string_without_database())
            .await?;
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database_name).as_str())
            .await?;
        // Migrate database
        let connection_pool = PgPool::connect(&settings.connection_string())
            .await?;
        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await?;

        let provider = Self {
            pool: connection_pool,
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
impl<GroupCiphertext, GroupCiphertextId> DsGenericStorageProvider<GroupCiphertext, GroupCiphertextId> for PostgresDsStorage 
where 
    GroupCiphertext: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Sync + Send,
    GroupCiphertextId: AsRef<[u8; 16]> + std::fmt::Debug + Sync + Send,
{
    type StorageError = PostgresStorageError;

    /// Loads the ds group state with the group ID.
    async fn load_group_state(&self, group_id: &GroupCiphertextId) -> Result<GenericLoadState<GroupCiphertext>, Self::StorageError> {
        let group_uuid = Uuid::from_bytes(group_id.as_ref().clone());

        let record = sqlx::query!(
            "SELECT ciphertext, last_used, deleted_queues FROM encrypted_groups WHERE group_id = $1",
            group_uuid 
        ).fetch_one(&self.pool).await.map_err(|e| {
            tracing::warn!("Error loading group state: {:?}", e);
            e 
        }
        )?;
        // A group id is reserved if it is in the database but has an empty
        // ciphertext.
        let ciphertext_bytes: Vec<u8> = record.ciphertext;
        let deleted_queues_bytes: Vec<u8> = record.deleted_queues;
        // TODO: We need a canonical version to signal if a group id is
        // reserved. I think we can just pass an option in, but then we'd have
        // to remove the NON NULL requirement for ciphertexts. Also leave a TODO
        // that there is probably a better way of doing this.
        
        if ciphertext_bytes.is_empty() {
            Ok(GenericLoadState::Reserved(TimeStamp::from(record.last_used))) } else {
                let ciphertext: GroupCiphertext = serde_json::from_slice(&ciphertext_bytes)?;
                let last_used = TimeStamp::from(record.last_used);
                if last_used.has_expired(GROUP_STATE_EXPIRATION_DAYS) {
                    Ok(GenericLoadState::Expired)
                } else {
                    Ok(GenericLoadState::Success(ciphertext))
                    }
    }
    }

    /// Saves the ds group state with the group ID.
    async fn save_group_state(
        &self,
        group_id: &GroupCiphertextId,
        encrypted_group_state: GroupCiphertext,
    ) -> Result<(), Self::StorageError> 
    where
        GroupCiphertext: 'async_trait, 
    {
        let group_uuid = Uuid::from_bytes(group_id.as_ref().clone());
        let last_used = Utc::now();
        let ciphertext = serde_json::to_vec(&encrypted_group_state)?;

        // Insert the group state into the database.
        sqlx::query!(
            r#"UPDATE encrypted_groups SET ciphertext = $2, last_used = $3 WHERE group_id = $1"#,
            group_uuid,
            ciphertext,
            last_used,
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
    async fn reserve_group_id(&self, group_id: &GroupCiphertextId) -> Result<bool, Self::StorageError> {
        let group_uuid = Uuid::from_bytes(group_id.as_ref().clone());

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

impl DsStorageProvider for PostgresDsStorage {}
