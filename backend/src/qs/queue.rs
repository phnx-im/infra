// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{codec::PhnxCodec, identifiers::QsClientId, messages::QueueMessage};
use sqlx::{Connection, PgConnection, PgExecutor, Row};

use crate::persistence::{QueueError, StorageError};

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

        // Update and get the sequence number
        let sequence_number_record = sqlx::query!(
            "UPDATE qs_queue_data 
            SET sequence_number = sequence_number + 1 
            WHERE queue_id = $1 
            RETURNING sequence_number - 1 as sequence_number",
            queue_id as &QsClientId,
        )
        .fetch_one(&mut *transaction)
        .await?;

        // Sequence number can't be NULL
        let sequence_number = sequence_number_record.sequence_number.unwrap();

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
            queue_id as &QsClientId,
            sequence_number,
            message_bytes,
        )
        .execute(&mut *transaction)
        .await?;

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
            .bind(queue_id)
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
            .collect::<Result<Vec<_>, QueueError>>()?;

        let remaining_messages = if let Some(row) = rows.first() {
            let remaining_count: i64 = row.try_get("count")?;
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
