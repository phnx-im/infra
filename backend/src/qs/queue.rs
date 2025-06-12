// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::{
    codec::{BlobDecoded, BlobEncoded},
    identifiers::QsClientId,
    messages::QueueMessage,
};
use sqlx::{Connection, PgConnection, PgExecutor};
use tokio_stream::StreamExt;
use tracing::info;

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
        message: &QueueMessage,
    ) -> Result<(), QueueError> {
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
            BlobEncoded(&message) as _,
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
        info!(sequence_number, "enqueue",);

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
                fetched.message_bytes AS "message: BlobDecoded<QueueMessage>",
                remaining.count
            FROM fetched, remaining
            "#,
            queue_id as &QsClientId,
            sequence_number as i64,
            number_of_messages,
        )
        .fetch(&mut *transaction);

        // Convert the records to messages.
        let mut remaining_count = None;
        let messages = rows
            .map(|row| {
                let row = row?;
                remaining_count.get_or_insert(row.count.unwrap_or_default());
                Ok(row.message.into_inner())
            })
            .collect::<Result<Vec<_>, QueueError>>()
            .await?;

        transaction.commit().await?;

        let remaining_messages = remaining_count
            .map(|count| count - messages.len() as i64)
            .unwrap_or_default()
            .try_into()
            .expect("logic error: negative remaining messages");

        info!(
            sequence_number,
            number_of_messages,
            num_messages = %messages.len(),
            remaining_messages,
            "read and delete"
        );

        Ok((messages, remaining_messages))
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

    #[cfg(test)]
    mod tests {
        use phnxcommon::crypto::ear::AeadCiphertext;
        use sqlx::PgPool;

        use crate::qs::{
            client_record::persistence::tests::store_random_client_record,
            user_record::persistence::tests::store_random_user_record,
        };

        use super::*;

        #[sqlx::test]
        async fn enqueue_read_and_delete(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(&pool, user_record.user_id).await?;

            let queue = Queue::new_and_store(client_record.client_id, &pool).await?;

            let n: u64 = queue.sequence_number.try_into()?;
            let mut messages = Vec::new();
            for sequence_number in n..n + 10 {
                let message = QueueMessage {
                    sequence_number,
                    ciphertext: AeadCiphertext::dummy(),
                };
                messages.push(message);
                Queue::enqueue(
                    pool.acquire().await?.as_mut(),
                    &client_record.client_id,
                    messages.last().unwrap(),
                )
                .await?;
            }

            let (loaded, remaining) = Queue::read_and_delete(
                pool.acquire().await?.as_mut(),
                &client_record.client_id,
                n + 1,
                5,
            )
            .await?;
            assert_eq!(loaded.len(), 5);
            assert_eq!(remaining, 4);
            assert_eq!(loaded, &messages[1..6]);

            let (loaded, remaining) = Queue::read_and_delete(
                pool.acquire().await?.as_mut(),
                &client_record.client_id,
                n + 1 + 5,
                5,
            )
            .await?;
            assert_eq!(loaded.len(), 4);
            assert_eq!(remaining, 0);
            assert_eq!(loaded, &messages[6..10]);

            Ok(())
        }
    }
}
