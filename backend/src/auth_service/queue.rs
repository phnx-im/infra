// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::AsClientId;
use sqlx::PgConnection;

use crate::errors::StorageError;

pub(super) struct Queue {
    queue_id: AsClientId,
    sequence_number: i64,
}

impl Queue {
    pub(super) async fn new_and_store(
        queue_id: AsClientId,
        connection: &mut PgConnection,
    ) -> Result<Self, StorageError> {
        let queue_data = Self {
            queue_id,
            sequence_number: 0,
        };
        queue_data.store(connection).await?;
        Ok(queue_data)
    }
}

mod persistence {
    use phnxtypes::{
        codec::persist::{BlobPersist, BlobPersisted},
        messages::QueueMessage,
    };
    use sqlx::Connection;
    use uuid::Uuid;

    use crate::errors::QueueError;

    use super::*;

    impl Queue {
        pub(super) async fn store(
            &self,
            connection: &mut PgConnection,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO as_queue_data (queue_id, sequence_number) VALUES ($1, $2)",
                self.queue_id.client_id(),
                self.sequence_number
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn enqueue(
            connection: &mut PgConnection,
            client_id: &AsClientId,
            message: QueueMessage,
        ) -> Result<(), QueueError> {
            // Begin the transaction
            let mut transaction = connection.begin().await?;

            // Check if sequence numbers are consistent.
            let sequence_number_record = sqlx::query!(
                "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1 FOR UPDATE",
                client_id.client_id(),
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

            // Get a fresh message ID (only used as a unique key for postgres)
            let message_id = Uuid::new_v4();
            // Store the message in the DB
            sqlx::query!(
                "INSERT INTO as_queues (message_id, queue_id, sequence_number, message_bytes)
                VALUES ($1, $2, $3, $4)",
                message_id,
                client_id.client_id(),
                sequence_number,
                message.persisting() as _,
            )
            .execute(&mut *transaction)
            .await?;

            let new_sequence_number = sequence_number + 1;
            // Increase the sequence number and store it.
            sqlx::query!(
                "UPDATE as_queue_data SET sequence_number = $2 WHERE queue_id = $1",
                client_id.client_id(),
                new_sequence_number
            )
            .execute(&mut *transaction)
            .await?;

            transaction.commit().await?;

            Ok(())
        }

        /// Delete all messages older than the given sequence number in the queue
        /// with the given client ID and return up to the requested number of
        /// messages from the queue starting with the message with the given
        /// sequence number, as well as the number of unread messages remaining in
        /// the queue.
        pub(in crate::auth_service) async fn read_and_delete(
            connection: &mut PgConnection,
            client_id: &AsClientId,
            sequence_number: u64,
            number_of_messages: u64,
        ) -> Result<(Vec<QueueMessage>, u64), QueueError> {
            let number_of_messages =
                i64::try_from(number_of_messages).map_err(|_| QueueError::LibraryError)?;

            let mut transaction = connection.begin().await?;

            // This query is idempotent, so there's no need to lock anything.
            let rows = sqlx::query!(
                r#"WITH deleted AS (
                DELETE FROM as_queues
                WHERE queue_id = $1 AND sequence_number < $2
            ),
            fetched AS (
                SELECT message_bytes FROM as_queues
                WHERE queue_id = $1 AND sequence_number >= $2
                ORDER BY sequence_number ASC
                LIMIT $3
            ),
            remaining AS (
                SELECT COUNT(*) AS count
                FROM as_queues
                WHERE queue_id = $1 AND sequence_number >= $2
            )
            SELECT
                fetched.message_bytes AS "message: BlobPersisted<QueueMessage>",
                remaining.count
            FROM fetched, remaining"#,
                client_id.client_id(),
                sequence_number as i64,
                number_of_messages,
            )
            .fetch_all(&mut *transaction)
            .await?;

            transaction.commit().await?;

            // Convert the records to messages.
            let mut remaining = None;
            let messages: Vec<QueueMessage> = rows
                .into_iter()
                .map(|row| {
                    remaining.get_or_insert(row.count);
                    let BlobPersisted(message) = row.message;
                    message
                })
                .collect();

            let remaining_messages = if let Some(remaining_count) = remaining.flatten() {
                // Subtract the number of messages we've read from the remaining
                // count to get the number of unread messages.
                remaining_count - messages.len() as i64
            } else {
                0
            };

            Ok((messages, remaining_messages as u64))
        }
    }
}
