// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, fmt, sync::Arc};

use displaydoc::Display;
use futures_util::stream;
use phnxcommon::identifiers::UserHandleHash;
use phnxprotos::auth_service::v1::{HandleQueueMessage, handle_queue_message};
use sqlx::{PgPool, postgres::PgListener};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;
use tonic::Status;
use tracing::error;
use uuid::Uuid;

use persistence::HandleQueue;

/// Maximum number of messages to fetch at once.
const MAX_BUFFER_SIZE: usize = 32;

/// Reliable, persistent message queue per user handle.
///
/// Allows for listening for new messages. Supports message acknowledgment to handle failures.
#[derive(Debug, Clone)]
pub(crate) struct UserHandleQueues {
    pool: PgPool,
    // Ensures that we have only a single stream per queue.
    listeners: Arc<Mutex<HashMap<UserHandleHash, CancellationToken>>>,
}

impl UserHandleQueues {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            pool,
            listeners: Default::default(),
        }
    }

    /// Returns a stream of messages from the queue specified by a user handle.
    ///
    /// This function continuously fetches messages from the queue. If the queue becomes empty, the
    /// stream will emit `None` and wait until a new message is added.
    ///
    /// Messages are identified by UUIDs. Messages are only removed from the queue once they are
    /// acknowledged.
    ///
    /// If another listener is already active for the same `hash`, that existing listener is
    /// cancelled before this new stream is returned. All messages that are not acknowledged will
    /// be emitted to the new listener.
    pub(crate) async fn listen(
        &self,
        hash: UserHandleHash,
    ) -> Result<impl Stream<Item = Option<HandleQueueMessage>> + use<>, HandleQueueError> {
        let mut pg_listener = PgListener::connect_with(&self.pool).await?;
        pg_listener
            .listen(&hash.pg_queue_label().to_string())
            .await?;

        let cancel = self.track_listener(hash).await?;
        let context = QueueStreamContext {
            id: Uuid::new_v4(),
            pool: self.pool.clone(),
            pg_listener,
            hash,
            cancel,
            buffer: Vec::with_capacity(MAX_BUFFER_SIZE),
            state: FetchState::Fetch,
        };
        Ok(context.into_stream())
    }

    /// Adds a message payload to the specified queue.
    ///
    /// If a listener is active for this `hash`, it will be notified that a new message is
    /// available to be fetched.
    ///
    /// A UUID will be assigned to the payload as message id and returned.
    pub(crate) async fn enqueue(
        &self,
        hash: &UserHandleHash,
        payload: handle_queue_message::Payload,
    ) -> Result<Uuid, HandleQueueError> {
        let mut txn = self.pool.begin().await?;

        let message_id = Uuid::new_v4();
        let message = HandleQueueMessage {
            message_id: Some(message_id.into()),
            payload: Some(payload),
        };

        HandleQueue::enqueue(txn.as_mut(), hash, message_id, message).await?;
        let query = format!(r#"NOTIFY "{}""#, hash.pg_queue_label());
        sqlx::query(&query).execute(txn.as_mut()).await?;

        txn.commit().await?;

        Ok(message_id)
    }

    /// Marks the message identified by `message_id` as acknowledged.
    ///
    /// Acknowledged messages are effectively removed from the queue.
    pub(crate) async fn ack(&self, message_id: Uuid) -> Result<(), HandleQueueError> {
        HandleQueue::delete(&self.pool, message_id).await?;
        Ok(())
    }

    async fn track_listener(&self, hash: UserHandleHash) -> sqlx::Result<CancellationToken> {
        let mut listeners = self.listeners.lock().await;
        listeners.retain(|_, cancel| !cancel.is_cancelled());
        let cancel = CancellationToken::new();
        if let Some(prev_cancel) = listeners.insert(hash, cancel.clone()) {
            prev_cancel.cancel();
        }
        Ok(cancel)
    }
}

trait UserHandleHashExt {
    fn pg_queue_label(&self) -> impl fmt::Display;
}

impl UserHandleHashExt for UserHandleHash {
    fn pg_queue_label(&self) -> impl fmt::Display {
        struct UserHandleHashDisplay<'a>(&'a UserHandleHash);

        impl fmt::Display for UserHandleHashDisplay<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "as_user_handle_queue_")?;
                for byte in self.0.as_bytes() {
                    write!(f, "{:02x}", byte)?;
                }
                Ok(())
            }
        }

        UserHandleHashDisplay(self)
    }
}

/// General error while accessing the requested queue.
#[derive(Debug, Error, Display)]
pub(crate) enum HandleQueueError {
    /// Database provider error
    Storage(#[from] sqlx::Error),
}

impl From<HandleQueueError> for Status {
    fn from(error: HandleQueueError) -> Self {
        let msg = error.to_string();
        match error {
            HandleQueueError::Storage(error) => {
                error!(%error, "storage error");
                Status::internal(msg)
            }
        }
    }
}

struct QueueStreamContext {
    id: Uuid,
    pool: PgPool,
    pg_listener: PgListener,
    hash: UserHandleHash,
    cancel: CancellationToken,
    /// Buffer for already fetched messages
    ///
    /// Note: the messages are stored in descending order.
    buffer: Vec<HandleQueueMessage>,
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
    fn into_stream(self) -> impl Stream<Item = Option<HandleQueueMessage>> + Send {
        stream::unfold(
            self,
            async |mut context| -> Option<(Option<HandleQueueMessage>, Self)> {
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
        HandleQueue::fetch_into(
            &self.pool,
            &self.hash,
            self.id,
            MAX_BUFFER_SIZE,
            &mut self.buffer,
        )
        .await
        .inspect_err(|error| {
            error!(%error, "failed to fetch next messages");
        })
        .ok()?;
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
    use prost::Message;
    use sqlx::{
        Database, Decode, Encode, PgExecutor, Postgres, Type, encode::IsNull, error::BoxDynError,
        query, query_scalar,
    };
    use tokio_stream::StreamExt;

    use super::*;

    pub(super) struct HandleQueue {}

    struct SqlHandleQueueMessage(HandleQueueMessage);

    impl Type<Postgres> for SqlHandleQueueMessage {
        fn type_info() -> <Postgres as Database>::TypeInfo {
            <Vec<u8> as Type<Postgres>>::type_info()
        }
    }

    impl<'q> Encode<'q, Postgres> for SqlHandleQueueMessage {
        fn encode_by_ref(
            &self,
            buf: &mut <Postgres as Database>::ArgumentBuffer<'q>,
        ) -> Result<IsNull, BoxDynError> {
            let buf: &mut Vec<u8> = buf.as_mut();
            self.0.encode(buf)?;
            Ok(IsNull::No)
        }
    }

    impl<'r> Decode<'r, Postgres> for SqlHandleQueueMessage {
        fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
            let bytes: &[u8] = Decode::<Postgres>::decode(value)?;
            let value = HandleQueueMessage::decode(bytes)?;
            Ok(SqlHandleQueueMessage(value))
        }
    }

    /// Fetches a messages into a buffer.
    ///
    /// The messages are fetched into the buffer in ascending order.
    impl HandleQueue {
        pub(crate) async fn fetch_into(
            executor: impl PgExecutor<'_>,
            hash: &UserHandleHash,
            fetched_by: Uuid,
            limit: usize,
            buffer: &mut Vec<HandleQueueMessage>,
        ) -> sqlx::Result<()> {
            let mut messages = query_scalar!(
                r#"WITH messages_to_fetch AS (
                    SELECT message_id FROM as_user_handles_queues
                    WHERE hash = $1 AND (fetched_by IS NULL OR fetched_by != $2)
                    ORDER BY created_at ASC
                    LIMIT $3
                    FOR UPDATE SKIP LOCKED
                )
                UPDATE as_user_handles_queues AS q
                SET fetched_by = $2
                FROM messages_to_fetch m
                WHERE q.message_id = m.message_id
                RETURNING q.message_bytes AS "message: SqlHandleQueueMessage""#,
                hash.as_bytes(),
                fetched_by,
                limit as i64,
            )
            .fetch(executor);
            while let Some(SqlHandleQueueMessage(message)) = messages.next().await.transpose()? {
                buffer.push(message);
            }
            Ok(())
        }

        pub(crate) async fn enqueue(
            executor: impl PgExecutor<'_>,
            hash: &UserHandleHash,
            message_id: Uuid,
            message: HandleQueueMessage,
        ) -> sqlx::Result<()> {
            debug_assert_eq!(Some(message_id.into()), message.message_id);
            query!(
                "INSERT INTO as_user_handles_queues (
                    message_id,
                    hash,
                    message_bytes
                ) VALUES ($1, $2, $3)",
                message_id,
                hash.as_bytes(),
                SqlHandleQueueMessage(message) as _,
            )
            .execute(executor)
            .await?;
            Ok(())
        }

        pub(crate) async fn delete(
            executor: impl PgExecutor<'_>,
            message_id: Uuid,
        ) -> sqlx::Result<()> {
            query!(
                "DELETE FROM as_user_handles_queues WHERE message_id = $1",
                message_id,
            )
            .execute(executor)
            .await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use std::{pin::pin, time};

    use phnxcommon::{
        credentials::keys::HandleVerifyingKey,
        time::{Duration, ExpirationData},
    };
    use phnxprotos::{
        auth_service::v1::{EncryptedConnectionOffer, handle_queue_message::Payload},
        common::v1::HpkeCiphertext,
    };
    use tokio::time::timeout;
    use tokio_stream::StreamExt;

    use crate::auth_service::user_handles::UserHandleRecord;

    use super::*;

    const STREAM_NEXT_TIMEOUT: time::Duration = time::Duration::from_secs(1);

    fn new_payload(payload_str: &str) -> Payload {
        Payload::ConnectionOffer(EncryptedConnectionOffer {
            ciphertext: Some(HpkeCiphertext {
                kem_output: b"kem_output".to_vec(),
                ciphertext: payload_str.as_bytes().to_vec(),
            }),
        })
    }

    fn msg(id: Uuid, payload: Payload) -> HandleQueueMessage {
        HandleQueueMessage {
            message_id: Some(id.into()),
            payload: Some(payload),
        }
    }

    async fn store_handle(pool: &PgPool, hash: UserHandleHash) -> anyhow::Result<()> {
        let hash_record = UserHandleRecord {
            user_handle_hash: hash,
            verifying_key: HandleVerifyingKey::from_bytes(vec![1]),
            expiration_data: ExpirationData::new(Duration::seconds(1)),
        };
        hash_record.store(pool).await?;
        Ok(())
    }

    #[test]
    fn pg_queue_label() {
        let lo: Uuid = "829e63e4-d6ed-4691-b8a3-f4bd17861505".parse().unwrap();
        let hi: Uuid = "c2cf7189-9250-49a5-b9c6-7b97ccc61ac8".parse().unwrap();
        let mut hash_bytes: [u8; 32] = [0; 32];
        hash_bytes[..16].copy_from_slice(lo.as_bytes());
        hash_bytes[16..].copy_from_slice(hi.as_bytes());
        let hash = UserHandleHash::new(hash_bytes);
        assert_eq!(
            hash.pg_queue_label().to_string(),
            "as_user_handle_queue_829e63e4d6ed4691b8a3f4bd17861505c2cf7189925049a5b9c67b97ccc61ac8"
        );
    }

    #[sqlx::test]
    async fn enqueue_fetch_delete_messages(pool: PgPool) -> anyhow::Result<()> {
        let hash = UserHandleHash::new([1; 32]);
        store_handle(&pool, hash).await?;

        let payload = new_payload("hello");

        let message_a_id = Uuid::new_v4();
        let message_b_id = Uuid::new_v4();

        let message_a = msg(message_a_id, payload.clone());
        let message_b = msg(message_b_id, payload.clone());

        HandleQueue::enqueue(&pool, &hash, message_a_id, message_a.clone()).await?;
        HandleQueue::enqueue(&pool, &hash, message_b_id, message_b.clone()).await?;

        let mut buffer = Vec::new();
        let fetched_by = Uuid::new_v4();

        HandleQueue::fetch_into(&pool, &hash, fetched_by, 1, &mut buffer).await?;
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], message_a, "First message should be fetched");

        HandleQueue::fetch_into(&pool, &hash, fetched_by, 1, &mut buffer).await?;
        assert_eq!(buffer.len(), 2, "Second message should be fetched");
        assert_eq!(buffer[1], message_b);

        HandleQueue::fetch_into(&pool, &hash, fetched_by, 1, &mut buffer).await?;
        assert_eq!(buffer.len(), 2, "No more messages should be fetched");

        let other_fetched_by = Uuid::new_v4();
        buffer.clear();
        HandleQueue::fetch_into(&pool, &hash, other_fetched_by, 100, &mut buffer).await?;
        assert_eq!(buffer.len(), 2, "All messages should be fetched again");
        assert_eq!(buffer[0], message_a);
        assert_eq!(buffer[1], message_b);

        HandleQueue::delete(&pool, message_a_id).await?;
        HandleQueue::delete(&pool, message_b_id).await?;

        let other_fetched_by = Uuid::new_v4();
        buffer.clear();
        HandleQueue::fetch_into(&pool, &hash, other_fetched_by, 100, &mut buffer).await?;
        assert_eq!(buffer.len(), 0, "No messages to fetch");

        Ok(())
    }

    #[sqlx::test]
    async fn enqueue_and_listen_single_message(pool: PgPool) {
        let hash = UserHandleHash::new([1; 32]);
        store_handle(&pool, hash).await.unwrap();
        let queues = UserHandleQueues::new(pool);

        let payload = new_payload("hello");
        let msg1_id = queues.enqueue(&hash, payload.clone()).await.unwrap();

        let mut stream = pin!(queues.listen(hash).await.unwrap());

        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .expect("Timeout waiting for message")
            .expect("Stream ended prematurely")
            .expect("Expected Some(QueueMessage), got None");

        assert_eq!(received_msg, msg(msg1_id, payload));

        // Check if queue is empty now for the listener (emits None)
        let next_item = timeout(STREAM_NEXT_TIMEOUT, stream.next()).await.unwrap();
        assert_eq!(
            next_item,
            Some(None),
            "Stream should yield Some(None) when queue is empty for the listener"
        );
    }

    #[sqlx::test]
    async fn listen_again_refetches_messages(pool: PgPool) {
        let hash = UserHandleHash::new([1; 32]);
        store_handle(&pool, hash).await.unwrap();
        let queues = UserHandleQueues::new(pool);

        let payload1 = new_payload("msg1");
        let payload2 = new_payload("msg2");
        let payload3 = new_payload("msg3");

        let msg1_id = queues.enqueue(&hash, payload1.clone()).await.unwrap();
        let msg2_id = queues.enqueue(&hash, payload2.clone()).await.unwrap();
        let msg3_id = queues.enqueue(&hash, payload3.clone()).await.unwrap();

        let mut stream = pin!(queues.listen(hash).await.unwrap());

        let received_msg1 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg1, msg(msg1_id, payload1.clone()));

        let received_msg2 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg2, msg(msg2_id, payload2));

        let received_msg3 = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg3, msg(msg3_id, payload3));

        // Listen again
        let mut stream = pin!(queues.listen(hash).await.unwrap());
        let first_after_relisten = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            first_after_relisten,
            msg(msg1_id, payload1),
            "Msg1 should be refetched again"
        );
    }

    #[sqlx::test]
    async fn ack_removes_messages(pool: PgPool) {
        let hash = UserHandleHash::new([1; 32]);
        store_handle(&pool, hash).await.unwrap();
        let queues = UserHandleQueues::new(pool);

        let payload1 = new_payload("msg1");
        let payload2 = new_payload("msg2");
        let payload3 = new_payload("msg3");

        let msg1_id = queues.enqueue(&hash, payload1.clone()).await.unwrap();
        let msg2_id = queues.enqueue(&hash, payload2.clone()).await.unwrap();
        let msg3_id = queues.enqueue(&hash, payload3.clone()).await.unwrap();

        queues.ack(msg2_id).await.unwrap(); // Ack msg2

        let mut stream = pin!(queues.listen(hash).await.unwrap());

        // Should only receive msg1 and msg3
        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg, msg(msg1_id, payload1));

        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg, msg(msg3_id, payload3));

        // No more messages
        let next_item = timeout(STREAM_NEXT_TIMEOUT, stream.next()).await.unwrap();

        assert_eq!(next_item, Some(None));
    }

    #[sqlx::test]
    async fn new_listener_cancels_previous_one(pool: PgPool) {
        let hash = UserHandleHash::new([1; 32]);
        store_handle(&pool, hash).await.unwrap();
        let queues = UserHandleQueues::new(pool);

        let payload1 = new_payload("msg1");

        let msg1_id = queues.enqueue(&hash, payload1.clone()).await.unwrap();

        let mut stream1 = pin!(queues.listen(hash).await.unwrap());

        // First listener gets the first message
        let received_msg1_listener1 = timeout(STREAM_NEXT_TIMEOUT, stream1.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received_msg1_listener1, msg(msg1_id, payload1));

        // Start a new listener for the same queue
        let _stream2 = queues.listen(hash).await.unwrap();

        // Try to get another message from stream1.
        // It should be cancelled, so it should yield None and then end.
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
    async fn listen_emits_none_when_empty_and_waits(pool: PgPool) {
        let hash = UserHandleHash::new([1; 32]);
        store_handle(&pool, hash).await.unwrap();
        let queues = UserHandleQueues::new(pool);

        let mut stream = pin!(queues.listen(hash).await.unwrap());

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
        let payload1 = new_payload("msg1");
        let msg1_id = queues.enqueue(&hash, payload1.clone()).await.unwrap();

        // Should receive the new message
        let received_msg = timeout(STREAM_NEXT_TIMEOUT, stream.next())
            .await
            .expect("Timeout waiting for new message")
            .expect("Stream ended prematurely after enqueue")
            .expect("Expected Some(QueueMessage) after enqueue, got None");
        assert_eq!(received_msg, msg(msg1_id, payload1));

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
        timeout(time::Duration::from_millis(50), stream.next())
            .await
            .expect_err("Stream should wait for the next message");
    }

    #[sqlx::test]
    async fn ack_non_existent_message(pool: PgPool) {
        let queues = UserHandleQueues::new(pool);
        let result = queues.ack(Uuid::new_v4()).await;
        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn enqueue_non_existent_queue(pool: PgPool) {
        let queues = UserHandleQueues::new(pool);
        let hash = UserHandleHash::new([1; 32]);

        let payload = new_payload("msg");
        let result = queues.enqueue(&hash, payload).await;
        assert!(result.is_err());
    }
}
