// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use core::ops::Add;

use async_trait::async_trait;
use num_traits::ToPrimitive;
use phnxbackend::qs::{
    client_record::QsClientRecord, storage_provider_trait::QsStorageProvider,
    user_record::QsUserRecord, QsConfig, QsSigningKey,
};
use phnxtypes::{
    crypto::hpke::ClientIdDecryptionKey,
    identifiers::{Fqdn, QsClientId, QsUserId},
    keypackage_batch::QsEncryptedAddPackage,
    messages::{FriendshipToken, QueueMessage},
};
use sqlx::{
    types::{BigDecimal, Uuid},
    PgPool,
};
use thiserror::Error;

#[derive(Debug)]
pub struct PostgresQsStorage {
    pool: PgPool,
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

    type LoadSigningKeyError = LoadSigningKeyError;
    type LoadDecryptionKeyError = LoadDecryptionKeyError;

    type LoadConfigError = LoadConfigError;

    async fn own_domain(&self) -> Fqdn {
        todo!()
    }

    async fn create_user(
        &self,
        user_record: QsUserRecord,
    ) -> Result<QsUserId, Self::CreateUserError> {
        todo!()
    }

    async fn load_user(&self, user_id: &QsUserId) -> Option<QsUserRecord> {
        todo!()
    }

    async fn store_user(
        &self,
        user_id: &QsUserId,
        user_record: QsUserRecord,
    ) -> Result<(), Self::StoreUserError> {
        todo!()
    }

    async fn delete_user(&self, user_id: &QsUserId) -> Result<(), Self::DeleteUserError> {
        todo!()
    }

    async fn create_client(
        &self,
        client_record: QsClientRecord,
    ) -> Result<QsClientId, Self::CreateClientError> {
        todo!()
    }

    async fn load_client(&self, client_id: &QsClientId) -> Option<QsClientRecord> {
        todo!()
    }

    async fn store_client(
        &self,
        client_id: &QsClientId,
        client_record: QsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        todo!()
    }

    async fn delete_client(&self, client_id: &QsClientId) -> Result<(), Self::DeleteClientError> {
        todo!()
    }

    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        todo!()
    }

    async fn store_last_resort_key_package(
        &self,
        client_id: &QsClientId,
        encrypted_key_package: QsEncryptedAddPackage,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        todo!()
    }

    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage> {
        todo!()
    }

    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Vec<QsEncryptedAddPackage> {
        todo!()
    }

    // TODO: The whole queueing scheme can probably be optimized quite a bit.
    async fn enqueue(
        &self,
        client_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError> {
        // Check if sequence numbers are consistent.
        let sequence_number_record = sqlx::query!(
            "SELECT sequence_number FROM queue_data WHERE queue_id = $1",
            client_id.as_uuid(),
        )
        .fetch_one(&self.pool)
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

        // Get a fresh message ID (only used as a unique key for postgres)
        let message_id = Uuid::new_v4();
        let message_bytes = serde_json::to_vec(&message)?;
        // Store the message in the DB
        sqlx::query!(
            "INSERT INTO queues (message_id, queue_id, sequence_number, message_bytes) VALUES ($1, $2, $3, $4)",
            message_id,
            client_id.as_uuid(),
            sequence_number_decimal,
            message_bytes,
        )
        .fetch_one(&self.pool)
        .await?;

        let new_sequence_number = sequence_number_decimal + BigDecimal::from(1u8);
        // Increase the sequence number and store it.
        sqlx::query!(
            r#"UPDATE queue_data SET sequence_number = $2 WHERE queue_id = $1"#,
            client_id.as_uuid(),
            new_sequence_number
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn read_and_delete(
        &self,
        client_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError> {
        let sequence_number_decimal = BigDecimal::from(sequence_number);
        // TODO: We can probably combine these three queries into one.

        // Delete all messages until the given "last seen" one.
        sqlx::query!(
            r#"DELETE FROM queues WHERE queue_id = $1 AND sequence_number <= $2"#,
            client_id.as_uuid(),
            sequence_number_decimal,
        )
        .execute(&self.pool)
        .await?;

        // Now fetch at most `number_of_messages` messages from the queue.

        // TODO: sqlx wants an i64 here and in a few other places below, but
        // we're using u64s. This is probably a limitation of postgres and we
        // might want to change some of the input/output types accordingly.
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| ReadAndDeleteError::LibraryError)?;
        let records = sqlx::query!(
            "SELECT message_bytes FROM queues WHERE queue_id = $1 ORDER BY sequence_number ASC LIMIT $2",
            client_id.as_uuid(),
            number_of_messages,
        )
        .fetch_all(&self.pool)
        .await?;

        let lower_limit = BigDecimal::from(sequence_number + records.len() as u64);
        let remaining_messages = sqlx::query!(
            "SELECT COUNT(*) as count FROM queues WHERE queue_id = $1 AND sequence_number >= $2 ",
            client_id.as_uuid(),
            lower_limit,
        )
        .fetch_one(&self.pool)
        .await?
        .count
        // Count should return something.
        .ok_or(ReadAndDeleteError::LibraryError)?;

        // Convert the records to messages.
        let messages = records
            .into_iter()
            .map(|record| {
                let message = serde_json::from_slice(&record.message_bytes)?;
                Ok(message)
            })
            .collect::<Result<Vec<_>, ReadAndDeleteError>>()?;

        return Ok((messages, remaining_messages as u64));
    }

    async fn load_signing_key(&self) -> Result<QsSigningKey, Self::LoadSigningKeyError> {
        todo!()
    }

    async fn load_decryption_key(
        &self,
    ) -> Result<ClientIdDecryptionKey, Self::LoadDecryptionKeyError> {
        todo!()
    }

    async fn load_config(&self) -> Result<QsConfig, Self::LoadConfigError> {
        todo!()
    }
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
}

#[derive(Error, Debug)]
#[repr(u8)]
pub enum CreateClientError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
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
pub enum LoadSigningKeyError {}

#[derive(Error, Debug)]
pub enum LoadDecryptionKeyError {}

#[derive(Error, Debug)]
pub enum LoadConfigError {}
