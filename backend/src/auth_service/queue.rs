// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Arc};

use futures_util::stream;
use phnxcommon::{identifiers::UserId, messages::QueueMessage};
use sqlx::{PgConnection, PgExecutor, PgPool, postgres::PgListener};
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::errors::{QueueError, StorageError};

/// Maximum number of messages to fetch at once.
const MAX_BUFFER_SIZE: usize = 32;

/// Reliable, persistent message queue.
///
/// Ensures messages are processed in order, supports message acknowledgment to handle failures,
/// and allows for continuous listening for new messages.
#[derive(Debug, Clone)]
pub(crate) struct Queues {
    pool: PgPool,
    // Ensures that we have only a single stream per queue.
    listeners: Arc<Mutex<HashMap<UserId, CancellationToken>>>,
}

impl Queues {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            pool,
            listeners: Default::default(),
        }
    }

    /// Returns a stream of messages from the specified queue.
    ///
    /// This function continuously fetches messages from the queue. If the queue becomes empty,
    /// the stream will emit `None` and wait until a new message is added.
    ///
    /// Messages are identified by contigious increasing sequence numbers. This stream will only
    /// fetch messages with a sequence number greater than or equal to `sequence_number_start`.
    /// Messages with sequence numbers less than `sequence_number_start` are implicitly
    /// acknowledged (i.e., considered processed and removed).
    ///
    /// If another listener is already active for the same `queue_id`, that existing listener
    /// is cancelled before this new stream is returned.
    pub(crate) async fn listen(
        &self,
        queue_id: &UserId,
        sequence_number_start: u64,
    ) -> Result<impl Stream<Item = Option<QueueMessage>> + Send + use<>, QueueError> {
        if !Queue::exists(&self.pool, queue_id).await? {
            return Err(QueueError::QueueNotFound);
        }

        let mut pg_listener = PgListener::connect_with(&self.pool).await?;
        pg_listener
            .listen(&format!("as_queue_{}", queue_id.uuid()))
            .await?;

        let cancel = self.track_listener(queue_id.clone()).await;
        if let Some(sequence_number) = sequence_number_start.checked_sub(1) {
            self.ack(queue_id, sequence_number).await?;
        }

        let context = QueueStreamContext {
            pool: self.pool.clone(),
            pg_listener,
            queue_id: queue_id.clone(),
            cancel: cancel.clone(),
            next_sequence_number: sequence_number_start,
            buffer: Vec::with_capacity(MAX_BUFFER_SIZE),
            state: FetchState::Fetch,
        };

        Ok(context.into_stream())
    }

    /// Adds a message to the specified queue.
    ///
    /// If a listener is active for this `queue_id`, it will be notified that a new message is
    /// available to be fetched.
    ///
    /// If the sequence number of the message is less than the sequence number of the last message
    /// in the queue, an error is returned.
    pub(crate) async fn enqueue(
        &self,
        queue_id: &UserId,
        message: &QueueMessage,
    ) -> Result<(), QueueError> {
        let mut transaction = self.pool.begin().await?;

        Queue::enqueue(&mut transaction, queue_id, message).await?;
        let query = format!(r#"NOTIFY "as_queue_{}""#, queue_id.uuid());
        sqlx::query(&query).execute(&mut *transaction).await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Marks messages in the specified queue as acknowledged up to and including the given
    /// `up_to_sequence_number`.
    ///
    /// Acknowledged messages are effectively removed from the queue.
    pub(crate) async fn ack(
        &self,
        queue_id: &UserId,
        up_to_sequence_number: u64,
    ) -> Result<(), QueueError> {
        if !Queue::exists(&self.pool, queue_id).await? {
            return Err(QueueError::QueueNotFound);
        }
        Queue::delete(&self.pool, queue_id, up_to_sequence_number).await?;
        Ok(())
    }

    async fn track_listener(&self, queue_id: UserId) -> CancellationToken {
        let mut listeners = self.listeners.lock().await;
        listeners.retain(|_, cancel| !cancel.is_cancelled());
        let cancel = CancellationToken::new();
        if let Some(prev_cancel) = listeners.insert(queue_id, cancel.clone()) {
            prev_cancel.cancel();
        }
        cancel
    }
}

pub(super) struct Queue<'a> {
    queue_id: &'a UserId,
    sequence_number: i64,
}

impl<'a> Queue<'a> {
    pub(super) async fn new_and_store(
        queue_id: &'a UserId,
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
    queue_id: UserId,
    cancel: CancellationToken,
    next_sequence_number: u64,
    /// Buffer for already fetched messages
    ///
    /// Note: the messages are stored in descending order.
    buffer: Vec<QueueMessage>,
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
        stream::unfold(
            self,
            async |mut context| -> Option<(Option<QueueMessage>, Self)> {
                loop {
                    if context.cancel.is_cancelled() {
                        return None;
                    }
                    if let Some(message) = context.buffer.pop() {
                        return Some((Some(message), context));
                    }
                    // buffer is empty
                    match context.state {
                        FetchState::Fetch => {
                            context.fetch_next_messages().await?;
                            if context.buffer.is_empty() {
                                // return sentinel value to indicate that the queue is empty
                                context.state = FetchState::Wait;
                                return Some((None, context));
                            }
                        }
                        FetchState::Wait => {
                            context.wait_for_notification().await?;
                            context.state = FetchState::Fetch;
                        }
                    }
                }
            },
        )
    }

    /// Fetches the next batch of messages into the internal buffer.
    async fn fetch_next_messages(&mut self) -> Option<()> {
        debug_assert!(self.buffer.is_empty());
        Queue::fetch_into(
            &self.pool,
            &self.queue_id,
            self.next_sequence_number,
            MAX_BUFFER_SIZE,
            &mut self.buffer,
        )
        .await
        .inspect_err(|error| {
            error!(%error, "failed to fetch next messages");
        })
        .ok()?;
        if let Some(message) = self.buffer.last() {
            self.next_sequence_number = message.sequence_number + 1;
        }
        self.buffer.reverse();
        Some(())
    }

    /// Waits for either a new message or for the listener to be cancelled.
    ///
    /// Returns `None` if the listener was cancelled and should stop.
    async fn wait_for_notification(&mut self) -> Option<()> {
        tokio::select! {
            _ = self.pg_listener.recv() => Some(()),
            _ = self.cancel.cancelled() => None,
        }
    }
}

mod persistence {
    use phnxcommon::{
        codec::{BlobDecoded, BlobEncoded},
        messages::QueueMessage,
    };
    use sqlx::{Connection, query, query_scalar};
    use tokio_stream::StreamExt;
    use uuid::Uuid;

    use crate::errors::QueueError;

    use super::*;

    impl Queue<'_> {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            query!(
                "INSERT INTO as_queue_data (queue_id, sequence_number) VALUES ($1, $2)",
                self.queue_id.uuid(),
                self.sequence_number
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(super) async fn exists(
            connection: impl PgExecutor<'_>,
            queue_id: &UserId,
        ) -> sqlx::Result<bool> {
            query_scalar!(
                "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1",
                queue_id.uuid()
            )
            .fetch_optional(connection)
            .await
            .map(|n| n.is_some())
        }

        pub(super) async fn enqueue(
            connection: &mut PgConnection,
            user_id: &UserId,
            message: &QueueMessage,
        ) -> Result<(), QueueError> {
            // Begin the transaction
            let mut transaction = connection.begin().await?;

            // Check if sequence numbers are consistent.
            let sequence_number = query_scalar!(
                "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1 FOR UPDATE",
                user_id.uuid(),
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
                user_id.uuid(),
                sequence_number,
                BlobEncoded(&message) as _,
            )
            .execute(&mut *transaction)
            .await?;

            let new_sequence_number = sequence_number + 1;
            // Increase the sequence number and store it.
            query!(
                "UPDATE as_queue_data SET sequence_number = $2 WHERE queue_id = $1",
                user_id.uuid(),
                new_sequence_number
            )
            .execute(&mut *transaction)
            .await?;

            transaction.commit().await?;

            Ok(())
        }

        /// Fetches a message with the given `sequence_number` into a buffer.
        ///
        /// `buffer` must be empty. The messages are fetched into the buffer in ascending order.
        pub(super) async fn fetch_into<'a>(
            executor: impl PgExecutor<'a> + 'a,
            user_id: &UserId,
            sequence_number: u64,
            limit: usize,
            buffer: &mut Vec<QueueMessage>,
        ) -> Result<(), QueueError> {
            let sequence_number: i64 = sequence_number
                .try_into()
                .map_err(|_| QueueError::LibraryError)?;
            let limit: i64 = limit.try_into().map_err(|_| QueueError::LibraryError)?;
            let mut messages = query_scalar!(
                r#"SELECT
                    message_bytes AS "message: BlobDecoded<QueueMessage>"
                FROM as_queues
                WHERE queue_id = $1 AND sequence_number >= $2
                ORDER BY sequence_number ASC
                FOR UPDATE SKIP LOCKED
                LIMIT $3"#,
                user_id.uuid(),
                sequence_number,
                limit,
            )
            .fetch(executor);
            while let Some(message) = messages.next().await {
                let BlobDecoded(message) = message?;
                buffer.push(message);
            }
            Ok(())
        }

        pub(super) async fn delete(
            connection: impl PgExecutor<'_>,
            user_id: &UserId,
            up_to_sequence_number: u64,
        ) -> Result<(), QueueError> {
            let up_to_sequence_number: i64 = up_to_sequence_number
                .try_into()
                .map_err(|_| QueueError::LibraryError)?;

            query!(
                r#"DELETE FROM as_queues
                WHERE queue_id = $1
                AND sequence_number <= $2"#,
                user_id.uuid(),
                up_to_sequence_number,
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

        use crate::auth_service::{
            client_record::persistence::tests::store_random_client_record,
            user_record::persistence::tests::store_random_user_record,
        };

        use super::*;

        #[sqlx::test]
        async fn enqueue_fetch_delete_and_requeue(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let user_id = user_record.user_id();
            store_random_client_record(&pool, user_id.clone()).await?;

            let queue = Queue::new_and_store(user_id, &pool).await?;

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
                    user_id,
                    messages.last().unwrap(),
                )
                .await?;
            }

            let mut buffer = Vec::new();
            Queue::fetch_into(&pool, user_id, 0, 10, &mut buffer).await?;
            assert_eq!(buffer.len(), 10);
            for i in 0..10 {
                assert_eq!(buffer[i], messages[i]);
            }

            buffer.clear();
            Queue::fetch_into(&pool, user_id, 10, 1, &mut buffer).await?;
            assert!(buffer.is_empty());

            Queue::delete(&pool, user_id, n + 4).await?;

            Queue::fetch_into(&pool, user_id, 5, 10, &mut buffer).await?;
            assert_eq!(buffer.len(), 5);
            for i in 0..5 {
                assert_eq!(buffer[i], messages[i + 5]);
            }

            buffer.clear();
            Queue::fetch_into(&pool, user_id, 10, 1, &mut buffer).await?;
            assert!(buffer.is_empty());

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::pin::pin;

    use phnxcommon::crypto::ear::AeadCiphertext;
    use tokio::time::{Duration, timeout};
    use tokio_stream::StreamExt;

    use crate::auth_service::{
        client_record::persistence::tests::store_random_client_record,
        user_record::persistence::tests::store_random_user_record,
    };

    use super::*;

    const STREAM_NEXT_TIMEOUT: Duration = Duration::from_secs(1);

    fn new_msg(seq: u64, payload_str: &str) -> QueueMessage {
        QueueMessage {
            sequence_number: seq,
            ciphertext: AeadCiphertext::new(payload_str.as_bytes().to_vec(), [0; 12]),
        }
    }

    async fn new_queue(pool: &PgPool) -> anyhow::Result<UserId> {
        let user_record = store_random_user_record(pool).await?;
        let user_id = user_record.user_id().clone();
        store_random_client_record(pool, user_id.clone()).await?;
        Queue::new_and_store(&user_id, pool).await?;
        Ok(user_id)
    }

    #[sqlx::test]
    async fn test_enqueue_and_listen_single_message(pool: PgPool) {
        let queue_id = new_queue(&pool).await.unwrap();
        let queues = Queues::new(pool);

        let msg1 = new_msg(0, "hello");
        queues.enqueue(&queue_id, &msg1).await.unwrap();

        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());

        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .expect("Timeout waiting for message")
            .expect("Stream ended prematurely")
            .expect("Expected Some(QueueMessage), got None");

        assert_eq!(received_msg, msg1);

        // Check if queue is empty now for the listener (emits None)
        let next_item = timeout(STREAM_NEXT_TIMEOUT, stream.next()).await.unwrap();
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

        let received_msg2 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg2, msg2);

        let received_msg3 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg3, msg3);

        // Listen again from 1, msg1 should be gone
        let mut stream = pin!(queues.listen(&queue_id, 0).await.unwrap());
        let first_after_relisten = timeout(STREAM_NEXT_TIMEOUT, stream.next())
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
        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg, msg3);

        // No more messages
        let next_item = timeout(STREAM_NEXT_TIMEOUT, stream.next()).await.unwrap();

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
        let received_msg1_listener1 = timeout(STREAM_NEXT_TIMEOUT, stream1.next())
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
        let cancellation_signal = timeout(STREAM_NEXT_TIMEOUT, stream1.next()).await;

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
        let item = timeout(STREAM_NEXT_TIMEOUT, stream.next())
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
        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .expect("Timeout waiting for new message")
            .expect("Stream ended prematurely after enqueue")
            .expect("Expected Some(QueueMessage) after enqueue, got None");
        assert_eq!(received_msg, msg1);

        // Queue is empty again for the listener
        let next_item = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .expect("Timeout waiting for new message")
            .expect("Stream ended prematurely after enqueue");
        assert_eq!(
            next_item, None,
            "Stream should yield None after consuming the message"
        );

        // Stream waits for the next message again
        timeout(Duration::from_millis(50), stream.next())
            .await
            .expect_err("Stream should wait for the next message");
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

        let recv_msg1 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(recv_msg1, msg1);

        let recv_msg2 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(recv_msg2, msg2);

        let recv_msg3 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(recv_msg3, msg3);

        let next_item = timeout(STREAM_NEXT_TIMEOUT, stream.next()).await.unwrap();
        assert_eq!(next_item, Some(None));
    }

    #[sqlx::test]
    async fn test_ack_non_existent_queue(pool: PgPool) {
        let queues = Queues::new(pool);
        let queue_id = UserId::random("localhost".parse().unwrap());

        let result = queues.ack(&queue_id, 0).await;

        assert!(matches!(result, Err(QueueError::QueueNotFound)));
    }

    #[sqlx::test]
    async fn test_enqueue_non_existent_queue(pool: PgPool) {
        let queues = Queues::new(pool);
        let queue_id = UserId::random("localhost".parse().unwrap());

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

        let received_msg3 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg3, msg3);

        let received_msg4 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
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
        let first_msg_stream_again = timeout(STREAM_NEXT_TIMEOUT, stream_again.next())
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
