// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{codec::PhnxCodec, identifiers::QsClientId, messages::QueueMessage};
use sqlx::{Connection, PgConnection, PgExecutor};

use crate::errors::{QueueError, StorageError};

pub(super) struct Queue {
    queue_id: QsClientId,
    sequence_number: i64,
}

impl Queue {
    pub(super) async fn new_and_store<'a>(
        queue_id: QsClientId,
        connection: impl PgExecutor<'_>,
    ) -> Result<Self, StorageError> {
        let queue_data = Self {
            queue_id,
            sequence_number: 0,
        };
        queue_data.store(connection).await?;
        Ok(queue_data)
    }

    pub(super) async fn enqueue(
        connection: &mut PgConnection,
        queue_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), QueueError> {
        // Encode the message
        let message_bytes = PhnxCodec::to_vec(&message)?;

        // Begin the transaction
        let mut transaction = connection.begin().await?;

        // Update and get the sequence number, saving one query
        let sequence_number = sqlx::query_scalar!(
            r#"
            WITH updated_sequence AS (
                -- Step 1: Update and return the current sequence number.
                UPDATE qs_queue_data 
                SET sequence_number = sequence_number + 1 
                WHERE queue_id = $1 
                RETURNING sequence_number - 1 as sequence_number
            )
            -- Step 2: Insert the message with the new sequence number.
            INSERT INTO qs_queues (queue_id, sequence_number, message_bytes) 
            SELECT $1, sequence_number, $2 FROM updated_sequence
            RETURNING sequence_number
            "#,
            queue_id as &QsClientId,
            message_bytes,
        )
        .fetch_one(&mut *transaction)
        .await?;

        // Check if the sequence number matches the one we got from the query. If it doesn't,
        // we return an error and automatically rollback the transaction.
        if sequence_number != message.sequence_number as i64 {
            tracing::warn!(
                "Sequence number mismatch. Message sequence number {}, queue sequence number {}",
                message.sequence_number,
                sequence_number
            );
            return Err(QueueError::SequenceNumberMismatch);
        }

        transaction.commit().await?;

        Ok(())
    }

    pub(super) async fn read_and_delete(
        connection: &mut PgConnection,
        queue_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), QueueError> {
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| QueueError::LibraryError)?;

        let mut transaction = connection.begin().await?;

        let rows = sqlx::query!(
            r#"
            WITH deleted AS (
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
                SELECT COALESCE(COUNT(*)) AS count 
                FROM qs_queues
                WHERE queue_id = $1 AND sequence_number >= $2
            )
            SELECT 
                fetched.message_bytes,
                remaining.count
            FROM fetched, remaining
            "#,
            queue_id as &QsClientId,
            sequence_number as i64,
            number_of_messages,
        )
        .fetch_all(&mut *transaction)
        .await?;

        transaction.commit().await?;

        // Convert the records to messages.
        let messages = rows
            .iter()
            .map(|row| {
                let message = PhnxCodec::from_slice(&row.message_bytes)?;
                Ok(message)
            })
            .collect::<Result<Vec<_>, QueueError>>()?;

        let remaining_messages = if let Some(row) = rows.first() {
            let remaining_count: i64 = row.count.unwrap_or_default();
            // Subtract the number of messages we've read from the remaining
            // count to get the number of unread messages.
            remaining_count - messages.len() as i64
        } else {
            0
        };

        Ok((messages, remaining_messages as u64))
    }
}

mod persistence {
    use super::*;

    impl Queue {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO 
                    qs_queue_data 
                    (queue_id, sequence_number)
                VALUES 
                    ($1, $2)",
                &self.queue_id as &QsClientId,
                self.sequence_number,
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }
}
