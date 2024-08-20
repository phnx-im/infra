// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use num_traits::ToPrimitive;
use phnxbackend::qs::{
    client_record::QsClientRecord, storage_provider_trait::QsStorageProvider,
    user_record::QsUserRecord, QsConfig, QsSigningKey,
};
use phnxtypes::{
    crypto::{
        errors::RandomnessError, hpke::ClientIdDecryptionKey, signatures::keys::QsUserVerifyingKey,
    },
    identifiers::{Fqdn, QsClientId, QsUserId},
    keypackage_batch::QsEncryptedAddPackage,
    messages::{FriendshipToken, QueueMessage},
    time::TimeStamp,
};
use sqlx::{
    postgres::PgArguments,
    types::{BigDecimal, Uuid},
    Arguments, PgPool, Row,
};
use thiserror::Error;

use crate::configurations::DatabaseSettings;

use super::connect_to_database;

#[derive(Debug)]
pub struct PostgresQsStorage {
    pool: PgPool,
    own_domain: Fqdn,
}

impl PostgresQsStorage {
    pub async fn new(
        settings: &DatabaseSettings,
        own_domain: Fqdn,
    ) -> Result<Self, CreateQsStorageError> {
        let pool = connect_to_database(settings).await?;

        let provider = Self { pool, own_domain };

        // Check if the database has been initialized.

        // TODO: This should probably go into its own function and be made more
        // explicit and robust.
        if provider.load_decryption_key().await.is_err() {
            provider.generate_fresh_decryption_key().await?;
        }
        if provider.load_signing_key().await.is_err() {
            provider.generate_fresh_signing_key().await?;
        }
        if provider.load_config().await.is_err() {
            provider
                .store_config(QsConfig {
                    domain: provider.own_domain.clone(),
                })
                .await?;
        }

        Ok(provider)
    }

    // The following functions should probably be part of the QS storage provider trait.
    // TODO: All the functions below use two queries. This can probably be optimized.

    async fn generate_fresh_signing_key(&self) -> Result<(), GenerateKeyError> {
        // Delete the existing key.
        sqlx::query!("DELETE FROM qs_signing_key")
            .execute(&self.pool)
            .await?;

        // Generate a new one and add it to the table
        let signing_key = QsSigningKey::generate()?;
        sqlx::query!(
            "INSERT INTO qs_signing_key (id, signing_key) VALUES ($1, $2)",
            Uuid::new_v4(),
            serde_json::to_vec(&signing_key)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn generate_fresh_decryption_key(&self) -> Result<(), GenerateKeyError> {
        // Delete the existing key.
        sqlx::query!("DELETE FROM qs_decryption_key")
            .execute(&self.pool)
            .await?;

        // Generate a new one and add it to the table
        let decryption_key = ClientIdDecryptionKey::generate()?;
        sqlx::query!(
            "INSERT INTO qs_decryption_key (id, decryption_key) VALUES ($1, $2)",
            Uuid::new_v4(),
            serde_json::to_vec(&decryption_key)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn store_config(&self, config: QsConfig) -> Result<(), StoreConfigError> {
        // Delete the existing config.
        sqlx::query!("DELETE FROM qs_config")
            .execute(&self.pool)
            .await?;

        // Store the new config.
        sqlx::query!(
            "INSERT INTO qs_config (id, config) VALUES ($1, $2)",
            Uuid::new_v4(),
            serde_json::to_vec(&config)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl QsStorageProvider for PostgresQsStorage {
    type EnqueueError = QueueError;
    type ReadAndDeleteError = ReadAndDeleteError;
    type CreateUserError = CreateUserError;
    type StoreUserError = StoreUserError;
    type DeleteUserError = DeleteUserError;
    type StoreClientError = StoreClientError;
    type CreateClientError = CreateClientError;
    type DeleteClientError = DeleteClientError;
    type StoreKeyPackagesError = StoreKeyPackagesError;
    type LoadUserKeyPackagesError = LoadUserKeyPackagesError;

    type LoadSigningKeyError = LoadSigningKeyError;
    type LoadDecryptionKeyError = LoadDecryptionKeyError;

    type LoadConfigError = LoadConfigError;

    async fn own_domain(&self) -> Fqdn {
        self.own_domain.clone()
    }

    async fn create_user(
        &self,
        user_record: QsUserRecord,
    ) -> Result<QsUserId, Self::CreateUserError> {
        let user_id = QsUserId::random();
        sqlx::query!(
            "INSERT INTO qs_user_records (user_id, friendship_token, verifying_key) VALUES ($1, $2, $3)", 
            user_id.as_uuid(),
            user_record.friendship_token().token(),
            user_record.verifying_key().as_ref(),
        )
        .execute(&self.pool)
        .await?;
        Ok(user_id)
    }

    async fn load_user(&self, user_id: &QsUserId) -> Option<QsUserRecord> {
        let user_record = sqlx::query!(
            "SELECT * FROM qs_user_records WHERE user_id = $1",
            user_id.as_uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;
        let qs_user_record = QsUserRecord::new(
            QsUserVerifyingKey::from_bytes(user_record.verifying_key),
            FriendshipToken::from_bytes(user_record.friendship_token),
        );
        Some(qs_user_record)
    }

    async fn store_user(
        &self,
        user_id: &QsUserId,
        user_record: QsUserRecord,
    ) -> Result<(), Self::StoreUserError> {
        sqlx::query!(
            "UPDATE qs_user_records SET friendship_token = $2, verifying_key = $3 WHERE user_id = $1",
            user_id.as_uuid(),
            user_record.friendship_token().token(),
            user_record.verifying_key().as_ref(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_user(&self, user_id: &QsUserId) -> Result<(), Self::DeleteUserError> {
        sqlx::query!(
            "DELETE FROM qs_user_records WHERE user_id = $1",
            user_id.as_uuid(),
        )
        .execute(&self.pool)
        .await?;

        // The ON DELETE CASCADE in the table creation statements take care of
        // deleting dependent client records, key packages, queue data and
        // queues.

        Ok(())
    }

    async fn create_client(
        &self,
        client_record: QsClientRecord,
    ) -> Result<QsClientId, Self::CreateClientError> {
        let client_id = QsClientId::random();

        // Create and store the client record.
        let encrypted_push_token = if let Some(ept) = client_record.encrypted_push_token() {
            Some(serde_json::to_vec(ept)?)
        } else {
            None
        };
        let owner_public_key = serde_json::to_vec(client_record.owner_public_key())?;
        let owner_signature_key = serde_json::to_vec(client_record.owner_signature_key())?;
        let ratchet = serde_json::to_vec(client_record.current_ratchet_key())?;
        let activity_time: &DateTime<Utc> = client_record.activity_time();

        let mut transaction = self.pool.begin().await?;

        sqlx::query!(
            "INSERT INTO qs_client_records (client_id, user_id, encrypted_push_token, owner_public_key, owner_signature_key, ratchet, activity_time) VALUES ($1, $2, $3, $4, $5, $6, $7)", 
            client_id.as_uuid(),
            client_record.user_id().as_uuid(),
            encrypted_push_token,
            owner_public_key,
            owner_signature_key,
            ratchet,
            activity_time,
        )
        .execute(&mut *transaction)
        .await?;

        // Initialize the client's queue
        sqlx::query!(
            "INSERT INTO qs_queue_data (queue_id, sequence_number) VALUES ($1, $2)",
            client_id.as_uuid(),
            BigDecimal::from(0u64),
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(client_id)
    }

    async fn load_client(&self, client_id: &QsClientId) -> Option<QsClientRecord> {
        let client_record = sqlx::query!(
            "SELECT * FROM qs_client_records WHERE client_id = $1",
            client_id.as_uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;
        let user_id = QsUserId::from(client_record.user_id);
        let encrypted_push_token = if let Some(ept) = client_record.encrypted_push_token {
            Some(serde_json::from_slice(&ept).ok()?)
        } else {
            None
        };
        let owner_public_key = serde_json::from_slice(&client_record.owner_public_key).ok()?;
        let owner_signature_key =
            serde_json::from_slice(&client_record.owner_signature_key).ok()?;
        let ratchet = serde_json::from_slice(&client_record.ratchet).ok()?;
        let activity_time = TimeStamp::from(client_record.activity_time);
        let result = QsClientRecord::from_db_values(
            user_id,
            encrypted_push_token,
            owner_public_key,
            owner_signature_key,
            ratchet,
            activity_time,
        );
        Some(result)
    }

    async fn store_client(
        &self,
        client_id: &QsClientId,
        client_record: QsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        let encrypted_push_token = if let Some(ept) = client_record.encrypted_push_token() {
            Some(serde_json::to_vec(ept)?)
        } else {
            None
        };
        let owner_public_key = serde_json::to_vec(client_record.owner_public_key())?;
        let owner_signature_key = serde_json::to_vec(client_record.owner_signature_key())?;
        let ratchet = serde_json::to_vec(client_record.current_ratchet_key())?;
        let activity_time: &DateTime<Utc> = client_record.activity_time();

        sqlx::query!(
            "UPDATE qs_client_records SET user_id = $2, encrypted_push_token = $3, owner_public_key = $4, owner_signature_key = $5, ratchet = $6, activity_time = $7 WHERE client_id = $1", 
            client_id.as_uuid(),
            client_record.user_id().as_uuid(),
            encrypted_push_token,
            owner_public_key,
            owner_signature_key,
            ratchet,
            activity_time,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_client(&self, client_id: &QsClientId) -> Result<(), Self::DeleteClientError> {
        sqlx::query!(
            "DELETE FROM qs_client_records WHERE client_id = $1",
            client_id.as_uuid(),
        )
        .execute(&self.pool)
        .await?;

        // The ON CASCADE actions in the respective table creation statements
        // take care of deletion of dependent key packages and queue data.

        Ok(())
    }

    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        // TODO: This can probably be improved. For now, we insert each key
        // package individually.
        store_key_packages(&self.pool, client_id, encrypted_key_packages, false).await?;
        Ok(())
    }

    async fn store_last_resort_key_package(
        &self,
        client_id: &QsClientId,
        encrypted_key_package: QsEncryptedAddPackage,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        store_key_packages(&self.pool, client_id, vec![encrypted_key_package], true).await?;
        Ok(())
    }

    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage> {
        // Check if the given client belongs to the given user.
        let _client_record = sqlx::query!(
            "SELECT * FROM qs_client_records WHERE client_id = $1 AND user_id = $2",
            client_id.as_uuid(),
            user_id.as_uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;

        let transaction = self.pool.begin().await.ok()?;

        // Lock the row s.t. it's not retrieved by another transaction.
        let add_package_record = sqlx::query!(
            "SELECT id, encrypted_add_package FROM key_packages WHERE client_id = $1 FOR UPDATE SKIP LOCKED",
            client_id.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        sqlx::query!(
            "DELETE FROM key_packages WHERE id = $1",
            add_package_record.id,
        )
        .execute(&self.pool)
        .await
        .ok()?;

        transaction.commit().await.ok()?;

        let result = serde_json::from_slice(&add_package_record.encrypted_add_package).ok()?;
        Some(result)
    }

    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Result<Vec<QsEncryptedAddPackage>, LoadUserKeyPackagesError> {
        // Figure out which user corresponds to the friendship token
        let user_record = sqlx::query!(
            "SELECT user_id FROM qs_user_records WHERE friendship_token = $1",
            friendship_token.token(),
        )
        .fetch_one(&self.pool)
        .await?;

        // TODO: This strategy isn't example optimal in terms of the time that
        // the KeyPackages of the clients are locked. I suspect that we can
        // optimize this by including a "FOR UPDATE SKIP LOCKED" in the
        // `selected_add_packages` query, if instead of filtering by `rn = ` we
        // sort by RN and then limit the number of rows to the number of
        // clients. That should skip any previously locked rows and leave all
        // non-chosesn rows open for locking.
        let query = "
        WITH client_ids AS (
            SELECT client_id FROM qs_client_records WHERE user_id = $1
        ),

        ranked_packages AS (
            SELECT p.id, p.encrypted_add_package, p.is_last_resort,
                   ROW_NUMBER() OVER (PARTITION BY p.client_id ORDER BY p.is_last_resort ASC) AS rn
            FROM key_packages p
            INNER JOIN client_ids c ON p.client_id = c.client_id
        ),

        selected_add_packages AS (
            SELECT id, encrypted_add_package
            FROM ranked_packages
            WHERE rn = 1
            FOR UPDATE
        ),

        deleted_packages AS (
            DELETE FROM key_packages
            WHERE id IN (SELECT id FROM selected_add_packages WHERE is_last_resort = FALSE)
            RETURNING encrypted_add_package
        )

        SELECT encrypted_add_package FROM selected_add_packages
        ";

        let mut transaction = self.pool.begin().await?;

        let rows = sqlx::query(query)
            .bind(user_record.user_id)
            .fetch_all(&mut *transaction)
            .await?;

        transaction.commit().await?;

        let encrypted_add_packages = rows
            .into_iter()
            .map(|row| {
                let encrypted_add_package_bytes = row
                    .try_get::<'_, Vec<u8>, _>("encrypted_add_package")
                    .map_err(|e| {
                        tracing::warn!("Error loading key package: {:?}", e);
                        LoadUserKeyPackagesError::PostgresError(e)
                    })?;
                let encrypted_add_package = serde_json::from_slice(
                    encrypted_add_package_bytes.as_slice(),
                )
                .map_err(|e| {
                    tracing::warn!("Error deserializing key package: {:?}", e);
                    LoadUserKeyPackagesError::SerializationError(e)
                })?;
                Ok(encrypted_add_package)
            })
            .collect::<Result<Vec<QsEncryptedAddPackage>, LoadUserKeyPackagesError>>()?;

        Ok(encrypted_add_packages)
    }

    async fn enqueue(
        &self,
        client_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError> {
        // Encode the message
        let message_bytes = serde_json::to_vec(&message)?;

        //tracing::info!("Encoded message: {:?}", message_bytes);

        // Begin the transaction
        let mut transaction = self.pool.begin().await?;

        // Check if sequence numbers are consistent.
        let sequence_number_record = sqlx::query!(
            "SELECT sequence_number FROM qs_queue_data WHERE queue_id = $1 FOR UPDATE",
            client_id.as_uuid(),
        )
        .fetch_one(&mut *transaction)
        .await?;

        // We're storing things as the NUMERIC postgres type. We need the
        // num-traits crate to convert to u64. If we find a better way to store
        // u64s, we might be able to get rid of that dependency.
        let sequence_number_decimal: BigDecimal = sequence_number_record.sequence_number;
        let sequence_number = sequence_number_decimal
            .to_u64()
            // The conversion should be successful, as we're only writing u64s
            // to the DB in the first place.
            .ok_or_else(|| QueueError::LibraryError)?;

        if sequence_number != message.sequence_number {
            tracing::warn!(
                "Sequence number mismatch. Message sequence number {}, queue sequence number {}",
                message.sequence_number,
                sequence_number
            );
            return Err(QueueError::SequenceNumberMismatch);
        }

        // Store the message in the DB
        sqlx::query!(
            "INSERT INTO qs_queues (queue_id, sequence_number, message_bytes) VALUES ($1, $2, $3)",
            client_id.as_uuid(),
            sequence_number_decimal,
            message_bytes,
        )
        .execute(&mut *transaction)
        .await?;

        let new_sequence_number = sequence_number_decimal + BigDecimal::from(1u8);
        // Increase the sequence number and store it.
        sqlx::query!(
            "UPDATE qs_queue_data SET sequence_number = $2 WHERE queue_id = $1",
            client_id.as_uuid(),
            new_sequence_number
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }

    async fn read_and_delete(
        &self,
        client_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError> {
        let sequence_number_decimal = BigDecimal::from(sequence_number);
        // TODO: sqlx wants an i64 here and in a few other places below, but
        // we're using u64s. This is probably a limitation of postgres and we
        // might want to change some of the input/output types accordingly.
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| ReadAndDeleteError::LibraryError)?;

        let mut transaction = self.pool.begin().await?;

        // This query is idempotent, so there's no need to lock anything.
        let query = "WITH deleted AS (
                DELETE FROM qs_queues 
                WHERE queue_id = $1 AND sequence_number < $2
                RETURNING *
            ),
            fetched AS (
                SELECT message_bytes FROM qs_queues
                WHERE queue_id = $1 AND sequence_number >= $2
                ORDER BY sequence_number ASC
                LIMIT $3
            ),
            remaining AS (
                SELECT COUNT(*) AS count 
                FROM qs_queues
                WHERE queue_id = $1 AND sequence_number >= $2
            )
            SELECT 
                fetched.message_bytes,
                remaining.count
            FROM fetched, remaining";

        let rows = sqlx::query(query)
            .bind(client_id.as_uuid())
            .bind(sequence_number_decimal)
            .bind(number_of_messages)
            .fetch_all(&mut *transaction)
            .await?;

        transaction.commit().await?;

        // Convert the records to messages.
        let messages = rows
            .iter()
            .map(|row| {
                let message_bytes: &[u8] = row.try_get("message_bytes")?;
                //tracing::info!("Message bytes: {:?}", message_bytes);
                let message = serde_json::from_slice(message_bytes)?;
                Ok(message)
            })
            .collect::<Result<Vec<_>, ReadAndDeleteError>>()?;

        tracing::info!("Read {} messages", messages.len());
        //tracing::info!("Messages: {:?}", messages);

        let remaining_messages = if let Some(row) = rows.first() {
            let remaining_count: i64 = row.try_get("count")?;
            // Subtract the number of messages we've read from the remaining
            // count to get the number of unread messages.
            remaining_count - messages.len() as i64
        } else {
            0
        };

        return Ok((messages, remaining_messages as u64));
    }

    async fn load_signing_key(&self) -> Result<QsSigningKey, Self::LoadSigningKeyError> {
        let signing_key_record = sqlx::query!("SELECT * FROM qs_signing_key",)
            .fetch_one(&self.pool)
            .await?;
        let signing_key = serde_json::from_slice(&signing_key_record.signing_key)?;
        Ok(signing_key)
    }

    async fn load_decryption_key(
        &self,
    ) -> Result<ClientIdDecryptionKey, Self::LoadDecryptionKeyError> {
        let decryption_key_record = sqlx::query!("SELECT * FROM qs_decryption_key",)
            .fetch_one(&self.pool)
            .await?;
        let decryption_key = serde_json::from_slice(&decryption_key_record.decryption_key)?;
        Ok(decryption_key)
    }

    async fn load_config(&self) -> Result<QsConfig, Self::LoadConfigError> {
        let config_record = sqlx::query!("SELECT * FROM qs_config",)
            .fetch_one(&self.pool)
            .await?;
        let config = serde_json::from_slice(&config_record.config)?;
        Ok(config)
    }
}

async fn store_key_packages(
    pool: &PgPool,
    client_id: &QsClientId,
    encrypted_add_packages: Vec<QsEncryptedAddPackage>,
    is_last_resort: bool,
) -> Result<(), StoreKeyPackagesError> {
    // TODO: This can probably be improved. For now, we insert each key
    // package individually.
    let client_uuid = client_id.as_uuid();

    let mut query_args = PgArguments::default();
    let mut query_string = String::from(
        "INSERT INTO key_packages (id, client_id, encrypted_add_package, is_last_resort) VALUES",
    );

    for (i, encrypted_add_package) in encrypted_add_packages.iter().enumerate() {
        let id = Uuid::new_v4();
        let encoded_add_package = serde_json::to_vec(encrypted_add_package)?;

        // Add values to the query arguments
        query_args.add(id);
        query_args.add(client_uuid);
        query_args.add(encoded_add_package);
        query_args.add(is_last_resort);

        if i > 0 {
            query_string.push(',');
        }

        // Add placeholders for each value
        query_string.push_str(&format!(
            " (${}, ${}, ${}, ${})",
            i * 4 + 1,
            i * 4 + 2,
            i * 4 + 3,
            i * 4 + 4
        ));
    }

    // Finalize the query string
    query_string.push(';');

    // Execute the query
    sqlx::query_with(&query_string, query_args)
        .execute(pool)
        .await?;

    Ok(())
}

#[derive(Error, Debug)]
#[repr(u8)]
pub enum StoreUserError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
}
#[derive(Error, Debug)]
pub enum DeleteUserError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
}
#[derive(Error, Debug)]
pub enum StoreClientError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error serializing client record
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
#[repr(u8)]
pub enum CreateClientError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
    /// Error serializing client record
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum DeleteClientError {
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
    /// Unknown client.
    #[error("Unknown client.")]
    UnknownClient,
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum StoreKeyPackagesError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Unknown client.
    #[error("Unknown client.")]
    UnknownClient,
    /// Error serializing KeyPackage
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum LoadUserKeyPackagesError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
    /// Error serializing KeyPackage
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}

/// Error creating user
#[derive(Error, Debug)]
pub enum CreateUserError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
}

/// Error creating queue
#[derive(Error, Debug)]
pub enum CreateQueueError {
    /// The given queue id collides with an existing one.
    #[error("The given queue id collides with an existing one.")]
    QueueIdCollision,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
}

/// General error while accessing the requested queue.
#[derive(Error, Debug)]
pub enum QueueError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Error serializing message
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}

/// Error while trying to read and delete messages from queue.
#[derive(Error, Debug)]
pub enum ReadAndDeleteError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing message
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
    /// A queue with the given id could not be found.
    #[error("The given queue id collides with an existing one.")]
    QueueNotFound,
    /// The given sequence number could not be found in the queue.
    #[error("The given sequence number could not be found in the queue.")]
    SequenceNumberNotFound,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
}

#[derive(Error, Debug)]
pub enum GenerateKeyError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
    #[error(transparent)]
    RandomnessError(#[from] RandomnessError),
}

#[derive(Error, Debug)]
pub enum LoadSigningKeyError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum LoadDecryptionKeyError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum StoreConfigError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum CreateQsStorageError {
    #[error(transparent)]
    StoreConfigError(#[from] StoreConfigError),
    #[error(transparent)]
    GenerateKeyError(#[from] GenerateKeyError),
    #[error(transparent)]
    StorageError(#[from] sqlx::Error),
    #[error(transparent)]
    MigrationError(#[from] sqlx::migrate::MigrateError),
}
