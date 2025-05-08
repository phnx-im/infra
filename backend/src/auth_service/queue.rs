// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Arc};

use phnxtypes::{identifiers::AsClientId, messages::QueueMessage};
use sqlx::{PgConnection, PgExecutor, PgPool, postgres::PgListener};
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::errors::{QueueError, StorageError};

/// Reliable persisted queues with pub/sub interface
///
/// Only a single listener is allowed per queue. Listening for new messages cancels the previous
/// listener.
#[derive(Debug, Clone)]
pub(crate) struct Queues {
    pool: PgPool,
    // Ensures that we have only a single worker per queue.
    workers: Arc<Mutex<HashMap<AsClientId, ExtendedDropGuard>>>,
}

impl Queues {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            pool,
            workers: Default::default(),
        }
    }

    pub(crate) async fn listen(
        &self,
        queue_id: &AsClientId,
        sequence_number_start: u64,
        tx: mpsc::Sender<Option<QueueMessage>>,
    ) -> Result<CancellationToken, QueueError> {
        info!(?self.workers, "listening to queue");

        // check if the queue exists
        Queue::sequence_number(&self.pool, queue_id)
            .await?
            .ok_or(QueueError::QueueNotFound)?;
        if let Some(sequence_number) = sequence_number_start.checked_sub(1) {
            self.ack(queue_id, sequence_number).await?;
        }

        let mut pg_listener = PgListener::connect_with(&self.pool).await?;
        pg_listener
            .listen(&format!("as_queue_{}", queue_id.client_id()))
            .await?;

        let cancel_worker = CancellationToken::new();
        self.track_worker(queue_id.clone(), cancel_worker.clone())
            .await;

        let worker = QueueWorker {
            pool: self.pool.clone(),
            pg_listener,
            queue_id: queue_id.clone(),
            tx,
            cancel: cancel_worker.clone(),
        };

        tokio::spawn(worker.run());

        Ok(cancel_worker)
    }

    async fn track_worker(&self, queue_id: AsClientId, cancel_worker: CancellationToken) {
        let mut workers = self.workers.lock().await;
        workers.retain(|_, cancel| !cancel.is_cancelled());
        workers.insert(queue_id, ExtendedDropGuard::new(cancel_worker));
    }

    pub(crate) async fn enqueue(
        &self,
        queue_id: &AsClientId,
        message: &QueueMessage,
    ) -> Result<(), QueueError> {
        let mut transaction = self.pool.begin().await?;

        Queue::enqueue(&mut transaction, queue_id, message).await?;
        let query = format!(r#"NOTIFY "as_queue_{}""#, queue_id.client_id());
        sqlx::query(&query).execute(&mut *transaction).await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Mark a message as acknowledged up to the given sequence number (inclusive).
    pub(crate) async fn ack(
        &self,
        queue_id: &AsClientId,
        up_to_sequence_number: u64,
    ) -> Result<(), QueueError> {
        Queue::delete(&self.pool, queue_id, up_to_sequence_number).await
    }
}

/// Like [`tokio_util::sync::DropGuard`] but allows to check if the token is cancelled.
#[derive(Debug)]
pub struct ExtendedDropGuard {
    pub(super) inner: Option<CancellationToken>,
}

impl ExtendedDropGuard {
    pub fn new(inner: CancellationToken) -> Self {
        Self { inner: Some(inner) }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner
            .as_ref()
            .map(|inner| inner.is_cancelled())
            .unwrap_or(true)
    }
}

impl Drop for ExtendedDropGuard {
    fn drop(&mut self) {
        if let Some(inner) = &self.inner {
            inner.cancel();
        }
    }
}

pub(super) struct Queue<'a> {
    queue_id: &'a AsClientId,
    sequence_number: i64,
}

impl<'a> Queue<'a> {
    pub(super) async fn new_and_store(
        queue_id: &'a AsClientId,
        connection: impl PgExecutor<'_>,
    ) -> Result<Self, StorageError> {
        let queue_data = Self {
            queue_id,
            sequence_number: 0,
        };
        queue_data.store(connection).await?;
        Ok(queue_data)
    }
}

struct QueueWorker {
    pool: PgPool,
    pg_listener: PgListener,
    queue_id: AsClientId,
    tx: mpsc::Sender<Option<QueueMessage>>,
    cancel: CancellationToken,
}

impl QueueWorker {
    async fn run(mut self) {
        if let Err(error) = self.try_run().await {
            error!(%error, "worker task failed");
        }
    }

    async fn try_run(&mut self) -> Result<(), QueueError> {
        // Requeue all messages with status `processing` that might habe been left behind by the
        // previous worker.
        Queue::requeue_processing_jobs(&self.pool, &self.queue_id).await?;
        loop {
            if !self.fetch_remaining_messages().await? {
                return Ok(()); // worker should stop
            }
            // Notify about the queue being empty.
            if self.tx.send(None).await.is_err() {
                return Ok(());
            }
            // wait either for a new message or for the working to be stopped
            tokio::select! {
                _ = self.pg_listener.recv() => {}
                _ = self.cancel.cancelled() => return Ok(()),
            }
        }
    }

    /// Returns `true` if the worker should continue listening for new messages.
    ///
    /// Returns `false` if the worker should stop listening for new messages. This happens if the
    /// channel is closed or if the worker is cancelled.
    async fn fetch_remaining_messages(&self) -> Result<bool, QueueError> {
        loop {
            if self.cancel.is_cancelled() {
                return Ok(false);
            }
            info!("fetching remain messages");
            let Some(message) = Queue::fetch(&self.pool, &self.queue_id).await? else {
                break;
            };
            info!(%message.sequence_number, "got message");
            if self.tx.send(Some(message.message.0)).await.is_err() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

mod persistence {
    use phnxtypes::{
        codec::{BlobDecoded, BlobEncoded},
        messages::QueueMessage,
    };
    use sqlx::{Connection, query, query_as, query_scalar};
    use uuid::Uuid;

    use crate::errors::QueueError;

    use super::*;

    pub(super) struct SqlQueueMessage {
        pub(super) sequence_number: i64,
        pub(super) message: BlobDecoded<QueueMessage>,
    }

    impl Queue<'_> {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            query!(
                "INSERT INTO as_queue_data (queue_id, sequence_number) VALUES ($1, $2)",
                self.queue_id.client_id(),
                self.sequence_number
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(super) async fn sequence_number(
            connection: impl PgExecutor<'_>,
            queue_id: &AsClientId,
        ) -> sqlx::Result<Option<u64>> {
            let n = query_scalar!(
                "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1",
                queue_id.client_id()
            )
            .fetch_optional(connection)
            .await?;
            Ok(n.and_then(|n| n.try_into().ok()))
        }

        pub(super) async fn enqueue(
            connection: &mut PgConnection,
            client_id: &AsClientId,
            message: &QueueMessage,
        ) -> Result<(), QueueError> {
            dbg!(client_id, message.sequence_number);

            // Begin the transaction
            let mut transaction = connection.begin().await?;

            // Check if sequence numbers are consistent.
            let sequence_number = query_scalar!(
                "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1 FOR UPDATE",
                client_id.client_id(),
            )
            .fetch_one(&mut *transaction)
            .await?;

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
            query!(
                "INSERT INTO as_queues (message_id, queue_id, sequence_number, message_bytes)
                VALUES ($1, $2, $3, $4)",
                message_id,
                client_id.client_id(),
                sequence_number,
                BlobEncoded(&message) as _,
            )
            .execute(&mut *transaction)
            .await?;

            let new_sequence_number = sequence_number + 1;
            // Increase the sequence number and store it.
            query!(
                "UPDATE as_queue_data SET sequence_number = $2 WHERE queue_id = $1",
                client_id.client_id(),
                new_sequence_number
            )
            .execute(&mut *transaction)
            .await?;

            transaction.commit().await?;

            Ok(())
        }

        /// Fetches a message with the lowest sequence number, s.t.
        ///
        /// * status is `pending`,
        /// * sequence number is >= `sequence_number` if `sequence_number` is not `None`.
        ///
        /// The message is set to have status `processing` before it is returned.
        pub(super) async fn fetch(
            pool: &PgPool,
            client_id: &AsClientId,
        ) -> Result<Option<SqlQueueMessage>, QueueError> {
            let mut transaction = pool.begin().await?;

            let message = query_as!(
                SqlQueueMessage,
                r#"SELECT
                    sequence_number,
                    message_bytes AS "message: _"
                FROM as_queues
                WHERE queue_id = $1 AND status = 'pending'
                ORDER BY sequence_number ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1"#,
                client_id.client_id(),
            )
            .fetch_optional(&mut *transaction)
            .await?;

            if let Some(message) = message.as_ref() {
                sqlx::query!(
                    "UPDATE as_queues SET status = 'processing'
                    WHERE queue_id = $1 and sequence_number = $2",
                    client_id.client_id(),
                    message.sequence_number,
                )
                .execute(&mut *transaction)
                .await?;
            }

            transaction.commit().await?;
            Ok(message)
        }

        pub(super) async fn requeue_processing_jobs(
            connection: impl PgExecutor<'_>,
            queue_id: &AsClientId,
        ) -> Result<(), QueueError> {
            query!(
                r#"UPDATE as_queues SET status = 'pending' WHERE queue_id = $1"#,
                queue_id.client_id()
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(super) async fn delete(
            connection: impl PgExecutor<'_>,
            client_id: &AsClientId,
            up_to_sequence_number: u64,
        ) -> Result<(), QueueError> {
            let up_to_sequence_number: i64 = up_to_sequence_number
                .try_into()
                .map_err(|_| QueueError::LibraryError)?;

            query!(
                r#"DELETE FROM as_queues
                WHERE queue_id = $1
                AND sequence_number <= $2"#,
                client_id.client_id(),
                up_to_sequence_number,
            )
            .execute(connection)
            .await?;

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use phnxtypes::crypto::ear::AeadCiphertext;
        use sqlx::PgPool;

        use crate::auth_service::{
            client_record::persistence::tests::store_random_client_record,
            user_record::persistence::tests::store_random_user_record,
        };

        use super::*;

        #[sqlx::test]
        async fn enqueue_fetch_delete_and_requeue(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_id = AsClientId::new(user_record.user_name().clone(), Uuid::new_v4());
            store_random_client_record(&pool, client_id.clone()).await?;

            let queue = Queue::new_and_store(&client_id, &pool).await?;

            let n: u64 = queue.sequence_number.try_into()?;
            let mut messages = Vec::new();
            for sequence_number in n..n + 10 {
                let message = QueueMessage {
                    sequence_number,
                    ciphertext: AeadCiphertext::random(),
                };
                messages.push(message);
                Queue::enqueue(
                    pool.acquire().await?.as_mut(),
                    &client_id,
                    messages.last().unwrap(),
                )
                .await?;
            }

            for i in 0..10 {
                let message = Queue::fetch(&pool, &client_id).await?.unwrap();
                assert_eq!(message.sequence_number as u64, n + i);
                assert_eq!(message.message.0, messages[i as usize]);
            }
            assert!(Queue::fetch(&pool, &client_id).await?.is_none());

            Queue::delete(&pool, &client_id, n + 4).await?;
            Queue::requeue_processing_jobs(&pool, &client_id).await?;

            for i in 5..10 {
                let message = Queue::fetch(&pool, &client_id).await?.unwrap();
                assert_eq!(message.sequence_number as u64, n + i);
                assert_eq!(message.message.0, messages[i as usize]);
            }
            assert!(Queue::fetch(&pool, &client_id).await?.is_none());

            Ok(())
        }
    }
}
