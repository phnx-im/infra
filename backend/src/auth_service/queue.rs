// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Arc};

use futures_util::stream;
use phnxtypes::{identifiers::AsClientId, messages::QueueMessage};
use sqlx::{PgConnection, PgExecutor, PgPool, postgres::PgListener};
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::errors::{QueueError, StorageError};

/// Reliable persisted queues with pub/sub interface
///
/// Only a single listener is allowed per queue. Listening for new messages cancels the previous
/// listener.
#[derive(Debug, Clone)]
pub(crate) struct Queues {
    pool: PgPool,
    // Ensures that we have only a stream per queue.
    listeners: Arc<Mutex<HashMap<AsClientId, ExtendedDropGuard>>>,
}

impl Queues {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            pool,
            listeners: Default::default(),
        }
    }

    pub(crate) async fn listen(
        &self,
        queue_id: &AsClientId,
        sequence_number_start: u64,
    ) -> Result<impl Stream<Item = Option<QueueMessage>> + Send + use<>, QueueError> {
        // check if the queue exists
        Queue::sequence_number(&self.pool, queue_id)
            .await?
            .ok_or(QueueError::QueueNotFound)?;

        let mut pg_listener = PgListener::connect_with(&self.pool).await?;
        pg_listener
            .listen(&format!("as_queue_{}", queue_id.client_id()))
            .await?;

        let cancel = self.track_listener(queue_id.clone()).await;
        if let Some(sequence_number) = sequence_number_start.checked_sub(1) {
            self.ack(queue_id, sequence_number).await?;
        }
        Queue::requeue_processing_jobs(&self.pool, &queue_id).await?;

        let context = QueueStreamContext {
            pool: self.pool.clone(),
            pg_listener,
            queue_id: queue_id.clone(),
            cancel: cancel.clone(),
            state: FetchState::Fetch,
        };

        Ok(context.into_stream())
    }

    async fn track_listener(&self, queue_id: AsClientId) -> CancellationToken {
        let mut workers = self.listeners.lock().await;
        workers.retain(|_, cancel| !cancel.is_cancelled());
        let cancel = CancellationToken::new();
        workers.insert(queue_id, ExtendedDropGuard::new(cancel.clone()));
        cancel
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
        let transaction = self.pool.begin().await?;
        // check if the queue exists
        Queue::sequence_number(&self.pool, queue_id)
            .await?
            .ok_or(QueueError::QueueNotFound)?;
        Queue::delete(&self.pool, queue_id, up_to_sequence_number).await?;
        transaction.commit().await?;
        Ok(())
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

struct QueueStreamContext {
    pool: PgPool,
    pg_listener: PgListener,
    queue_id: AsClientId,
    cancel: CancellationToken,
    state: FetchState,
}

enum FetchState {
    /// Fetch the next message.
    Fetch,
    /// Wait for a notification to fetch the next message.
    ///
    /// This state is used when the queue is empty.
    Wait,
}

impl QueueStreamContext {
    fn into_stream(self) -> impl Stream<Item = Option<QueueMessage>> + Send {
        stream::unfold(self, async |mut context| {
            if context.cancel.is_cancelled() {
                return None;
            }
            loop {
                match context.state {
                    FetchState::Fetch => {
                        let message = match context.fetch_next_message().await {
                            Ok(message) => message,
                            Err(error) => {
                                error!(%error, "failed to fetch next message");
                                return None;
                            }
                        };
                        if message.is_none() {
                            context.state = FetchState::Wait;
                        }
                        return Some((message, context));
                    }
                    FetchState::Wait => {
                        if !context.wait().await {
                            return None;
                        }
                        context.state = FetchState::Fetch;
                    }
                }
            }
        })
    }

    async fn fetch_next_message(&self) -> Result<Option<QueueMessage>, QueueError> {
        let sql_message = Queue::fetch(&self.pool, &self.queue_id).await?;
        Ok(sql_message.map(|message| message.message.0))
    }

    /// Waits for either a new message or for the worker to be cancelled.
    ///
    /// Returns `true` if the worker should continue listening for new messages.
    async fn wait(&mut self) -> bool {
        // wait either for a new message or for the working to be stopped
        tokio::select! {
            _ = self.pg_listener.recv() => true,
            _ = self.cancel.cancelled() => false,
        }
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
            // Begin the transaction
            let mut transaction = connection.begin().await?;

            // Check if sequence numbers are consistent.
            let sequence_number = query_scalar!(
                "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1 FOR UPDATE",
                client_id.client_id(),
            )
            .fetch_one(&mut *transaction)
            .await;
            let sequence_number = match sequence_number {
                Ok(sequence_number) => sequence_number,
                Err(sqlx::Error::RowNotFound) => return Err(QueueError::QueueNotFound),
                Err(error) => return Err(error.into()),
            };

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

#[cfg(test)]
mod tests {
    use std::pin::pin;

    use phnxtypes::crypto::ear::AeadCiphertext;
    use tokio::time::{Duration, timeout};
    use tokio_stream::StreamExt;
    use uuid::Uuid;

    use crate::auth_service::{
        client_record::persistence::tests::store_random_client_record,
        user_record::persistence::tests::store_random_user_record,
    };

    use super::*;

    fn new_msg(seq: u64, payload_str: &str) -> QueueMessage {
        QueueMessage {
            sequence_number: seq,
            ciphertext: AeadCiphertext::new(payload_str.as_bytes().to_vec(), [0; 12]),
        }
    }

    async fn new_queue(pool: &PgPool) -> anyhow::Result<AsClientId> {
        let user_record = store_random_user_record(pool).await?;
        let client_id = AsClientId::new(user_record.user_name().clone(), Uuid::new_v4());
        store_random_client_record(pool, client_id.clone()).await?;
        Queue::new_and_store(&client_id, pool).await?;
        Ok(client_id)
    }

    #[sqlx::test]
    async fn test_enqueue_and_listen_single_message(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "hello");
        queues.enqueue(&queue_id, &msg1).await.unwrap();

        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());

        let received_msg = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("Timeout waiting for message")
            .expect("Stream ended prematurely")
            .expect("Expected Some(QueueMessage), got None");

        assert_eq!(received_msg, msg1);

        // Check if queue is empty now for the listener (emits None)
        let next_item = timeout(Duration::from_millis(50), stream.next()) // Short timeout
            .await
            .ok() // It's ok to timeout here, means no immediate message
            .flatten();
        assert_eq!(
            next_item,
            Some(None),
            "Stream should yield Some(None) when queue is empty for the listener"
        );
    }

    #[sqlx::test]
    async fn test_listen_respects_sequence_number_start(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "msg1");
        let msg2 = new_msg(1, "msg2");
        let msg3 = new_msg(2, "msg3");

        queues.enqueue(&queue_id, &msg1).await.unwrap();
        queues.enqueue(&queue_id, &msg2).await.unwrap();
        queues.enqueue(&queue_id, &msg3).await.unwrap();

        // Listen starting from sequence number 2
        let mut stream = pin!(queues.listen(&queue_id, 1).await.unwrap());

        let received_msg2 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg2, msg2);

        let received_msg3 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg3, msg3);

        // Listen again from 1, msg1 should be gone
        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());
        let first_after_relisten = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            first_after_relisten, msg2,
            "Msg1 should have been acked by previous listen call starting at 2"
        );
    }

    #[sqlx::test]
    async fn test_ack_removes_messages(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "msg1");
        let msg2 = new_msg(1, "msg2");
        let msg3 = new_msg(2, "msg3");

        queues.enqueue(&queue_id, &msg1).await.unwrap();
        queues.enqueue(&queue_id, &msg2).await.unwrap();
        queues.enqueue(&queue_id, &msg3).await.unwrap();

        queues.ack(&queue_id, 1).await.unwrap(); // Ack up to msg2

        // Listen from the beginning (seq 0 or 1)
        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());

        // Should only receive msg3
        let received_msg = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg, msg3);

        // No more messages
        let next_item = timeout(Duration::from_millis(50), stream.next())
            .await
            .ok()
            .flatten();
        assert_eq!(next_item, Some(None));
    }

    #[sqlx::test]
    async fn test_new_listener_cancels_previous_one(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "msg1");

        queues.enqueue(&queue_id, &msg1).await.unwrap();

        let mut stream1 = pin!(queues.listen(&queue_id, 0).await.unwrap());

        // First listener gets the first message
        let received_msg1_listener1 = timeout(Duration::from_secs(1), stream1.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg1_listener1, msg1);

        // Start a new listener for the same queue
        let _stream2 = queues.listen(&queue_id, 0).await.unwrap();

        // Try to get another message from stream1.
        // It should be cancelled, so it should yield None and then end.
        // The mock implementation sends a single None upon cancellation.
        let cancellation_signal = timeout(Duration::from_secs(1), stream1.next()).await;

        match cancellation_signal {
            Ok(None) => { /* Expected cancellation signal */ }
            Ok(Some(m)) => {
                panic!("Stream1 should have been cancelled, but received message: {m:?}")
            }
            Err(_) => panic!("Timeout waiting for stream1 to be cancelled"),
        }
    }

    #[sqlx::test]
    async fn test_listen_emits_none_when_empty_and_waits(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());

        // Initially empty, should yield None
        let item = timeout(Duration::from_millis(100), stream.next()) // Increased timeout slightly
            .await
            .expect("Timeout waiting for initial None")
            .expect("Stream should not end immediately");
        assert_eq!(
            item, None,
            "Stream should yield None when queue is initially empty"
        );

        // Enqueue a message
        let msg1 = new_msg(0, "new_message");
        queues.enqueue(&queue_id, &msg1).await.unwrap();

        // Should receive the new message
        let received_msg = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("Timeout waiting for new message")
            .expect("Stream ended prematurely after enqueue")
            .expect("Expected Some(QueueMessage) after enqueue, got None");
        assert_eq!(received_msg, msg1);

        // Queue is empty again for the listener
        let next_item = timeout(Duration::from_millis(100), stream.next())
            .await
            .expect("Timeout waiting for new message")
            .expect("Stream ended prematurely after enqueue");
        assert_eq!(
            next_item, None,
            "Stream should yield None after consuming the message"
        );

        // Stream waits for the next message again
        let next_item = timeout(Duration::from_millis(50), stream.next()).await.ok();
        assert_eq!(next_item, None, "Stream should wait for the next message");
    }

    #[sqlx::test]
    async fn test_multiple_messages_are_received_in_order(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "msg1");
        let msg2 = new_msg(1, "msg2");
        let msg3 = new_msg(2, "msg3");

        queues.enqueue(&queue_id, &msg1).await.unwrap();
        queues.enqueue(&queue_id, &msg2).await.unwrap();
        queues.enqueue(&queue_id, &msg3).await.unwrap();

        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());

        let recv_msg1 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(recv_msg1, msg1);

        let recv_msg2 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(recv_msg2, msg2);

        let recv_msg3 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(recv_msg3, msg3);

        let next_item = timeout(Duration::from_millis(50), stream.next())
            .await
            .ok()
            .flatten();
        assert_eq!(next_item, Some(None));
    }

    #[sqlx::test]
    async fn test_ack_non_existent_queue(pool: PgPool) {
        let queues = Queues::new(pool);
        let queue_id = AsClientId::new("alice@localhost".parse().unwrap(), Uuid::new_v4());

        let result = queues.ack(&queue_id, 0).await;

        assert!(matches!(result, Err(QueueError::QueueNotFound)));
    }

    #[sqlx::test]
    async fn test_enqueue_non_existent_queue(pool: PgPool) {
        let queues = Queues::new(pool);
        let queue_id = AsClientId::new("alice@localhost".parse().unwrap(), Uuid::new_v4());

        let msg = new_msg(0, "msg");
        let result = queues.enqueue(&queue_id, &msg).await;

        assert!(matches!(result, Err(QueueError::QueueNotFound)));
    }

    #[sqlx::test]
    async fn test_listen_acknowledges_past_messages_on_start(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "past_msg1");
        let msg2 = new_msg(1, "past_msg2");
        let msg3 = new_msg(2, "current_msg3");
        let msg4 = new_msg(3, "future_msg4");

        queues.enqueue(&queue_id, &msg1).await.unwrap();
        queues.enqueue(&queue_id, &msg2).await.unwrap();
        queues.enqueue(&queue_id, &msg3).await.unwrap();
        queues.enqueue(&queue_id, &msg4).await.unwrap();

        // Listen starting from sequence number 2. Messages 1 and 2 should be acknowledged.
        let mut stream = pin!(queues.listen(&queue_id, 2).await.unwrap());

        let received_msg3 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg3, msg3);

        let received_msg4 = timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg4, msg4);

        // If we were to ack msg2, then listen from 1, we should only get msg3, msg4
        // But listen(2) already did this. Let's try acking something already acked.
        queues.ack(&queue_id, 1).await.unwrap(); // Should be idempotent or no-op for already acked

        // Listen again from a lower number, e.g. 0. Since msgs 1,2 were acked by listen(2),
        // they should still be gone.
        let mut stream_again = pin!(queues.listen(&queue_id, 0).await.unwrap());
        let first_msg_stream_again = timeout(Duration::from_secs(1), stream_again.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            first_msg_stream_again, msg3,
            "Messages 1 and 2 should remain acknowledged"
        );
    }
}
