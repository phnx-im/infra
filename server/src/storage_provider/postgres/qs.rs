// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use phnxbackend::qs::{
    client_record::QsClientRecord, storage_provider_trait::QsStorageProvider, QsSigningKey,
};
use phnxtypes::{
    codec::PhnxCodec,
    crypto::{
        errors::RandomnessError, hpke::ClientIdDecryptionKey, signatures::keys::QsUserVerifyingKey,
    },
    identifiers::{Fqdn, QsClientId, QsUserId},
    keypackage_batch::QsEncryptedAddPackage,
    messages::{FriendshipToken, QueueMessage},
    time::TimeStamp,
};
use sqlx::{postgres::PgArguments, types::Uuid, Arguments, PgPool, Row};
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
            PhnxCodec::to_vec(&signing_key)?,
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
            PhnxCodec::to_vec(&decryption_key)?,
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
    type StoreKeyPackagesError = StoreKeyPackagesError;
    type LoadUserKeyPackagesError = LoadUserKeyPackagesError;

    type LoadSigningKeyError = LoadSigningKeyError;
    type LoadDecryptionKeyError = LoadDecryptionKeyError;

    type LoadConfigError = LoadConfigError;

    async fn own_domain(&self) -> Fqdn {
        self.own_domain.clone()
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

        let result = PhnxCodec::from_slice(&add_package_record.encrypted_add_package).ok()?;
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
                let encrypted_add_package =
                    PhnxCodec::from_slice(encrypted_add_package_bytes.as_slice()).map_err(|e| {
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
        let message_bytes = PhnxCodec::to_vec(&message)?;

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
        let sequence_number = sequence_number_record.sequence_number;

        if sequence_number != message.sequence_number as i64 {
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
            sequence_number,
            message_bytes,
        )
        .execute(&mut *transaction)
        .await?;

        let new_sequence_number = sequence_number + 1;
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
            .bind(sequence_number as i64)
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
                let message = PhnxCodec::from_slice(message_bytes)?;
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
        let signing_key = PhnxCodec::from_slice(&signing_key_record.signing_key)?;
        Ok(signing_key)
    }

    async fn load_decryption_key(
        &self,
    ) -> Result<ClientIdDecryptionKey, Self::LoadDecryptionKeyError> {
        let decryption_key_record = sqlx::query!("SELECT * FROM qs_decryption_key",)
            .fetch_one(&self.pool)
            .await?;
        let decryption_key = PhnxCodec::from_slice(&decryption_key_record.decryption_key)?;
        Ok(decryption_key)
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
        let encoded_add_package = PhnxCodec::to_vec(encrypted_add_package)?;

        // Add values to the query arguments. None of these should throw an error.
        let _ = query_args.add(id);
        let _ = query_args.add(client_uuid);
        let _ = query_args.add(encoded_add_package);
        let _ = query_args.add(is_last_resort);

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
    SerializationError(#[from] phnxtypes::codec::Error),
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
    SerializationError(#[from] phnxtypes::codec::Error),
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
    SerializationError(#[from] phnxtypes::codec::Error),
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
    SerializationError(#[from] phnxtypes::codec::Error),
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
    SerializationError(#[from] phnxtypes::codec::Error),
}

/// Error while trying to read and delete messages from queue.
#[derive(Error, Debug)]
pub enum ReadAndDeleteError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing message
    #[error(transparent)]
    DeserializationError(#[from] phnxtypes::codec::Error),
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
    DeserializationError(#[from] phnxtypes::codec::Error),
    #[error(transparent)]
    RandomnessError(#[from] RandomnessError),
}

#[derive(Error, Debug)]
pub enum LoadSigningKeyError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] phnxtypes::codec::Error),
}

#[derive(Error, Debug)]
pub enum LoadDecryptionKeyError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] phnxtypes::codec::Error),
}

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] phnxtypes::codec::Error),
}

#[derive(Error, Debug)]
pub enum StoreConfigError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Error deserializing key
    #[error(transparent)]
    SerializationError(#[from] phnxtypes::codec::Error),
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
