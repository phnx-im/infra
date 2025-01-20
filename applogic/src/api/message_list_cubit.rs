// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, pin::pin, sync::Arc};

use flutter_rust_bridge::frb;
use phnxcoreclient::{
    store::{Store, StoreEntityId, StoreNotification, StoreOperation},
    ConversationId, ConversationMessage,
};
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    util::{spawn_from_sync, Cubit, CubitCore},
    StreamSink,
};

use super::{
    types::{UiConversationMessage, UiConversationMessageId, UiFlightPosition},
    user_cubit::UserCubitBase,
};

/// The state reprensenting a list of messages in a conversation.
///
/// The state is cheaply clonable (internally reference counted).
#[frb(opaque)]
#[derive(Debug, Default, Clone)]
pub struct MessageListState {
    /// Copy-on-write inner ref to make the state cheaply clonable when emitting new state
    inner: Arc<MessageListStateInner>,
}

#[frb(ignore)]
#[derive(Debug, Default)]
struct MessageListStateInner {
    /// Loaded messages (not all messages in the conversation)
    messages: Vec<UiConversationMessage>,
    /// Lookup index mapping a message id to the index in `messages`
    message_ids_index: HashMap<UiConversationMessageId, usize>,
}

impl MessageListState {
    /// Rebuild the state from loaded messages
    ///
    /// `include_first` indicates whether the first message should be included in the loaded
    /// messages. In case it is *NOT* included, it is used only to calculate the flight position of
    /// the second message, and is discarded.
    ///
    /// The state is fully replaced. Note: This behavior will change when we will introduce loading
    /// of additional messages via batching <https://github.com/phnx-im/infra/issues/287>.
    fn rebuild_from_messages(
        &mut self,
        mut new_messages: Vec<ConversationMessage>,
        include_first: bool,
    ) {
        let capacity = new_messages.len().saturating_sub(1);
        let mut messages = Vec::with_capacity(capacity);
        let mut message_ids_index = HashMap::with_capacity(capacity);

        let mut messages_iter = new_messages.drain(..);

        let prev = if include_first {
            None
        } else {
            messages_iter.next().map(UiConversationMessage::from)
        };
        let mut prev = prev.as_ref();
        let mut cur = messages_iter.next().map(UiConversationMessage::from);

        while let Some(mut message) = cur.take() {
            let next = messages_iter.next().map(From::from);

            message.position = UiFlightPosition::calculate(&message, prev, next.as_ref());

            message_ids_index.insert(message.id, messages.len());
            messages.push(message);

            prev = messages.last();
            cur = next;
        }

        let inner = MessageListStateInner {
            message_ids_index,
            messages,
        };
        self.inner = Arc::new(inner); // copy on write
    }

    /// The number of loaded messages in the list.
    ///
    /// Note that this is not the number of all messages in the conversation.
    #[frb(sync, getter, type_64bit_int)]
    pub fn loaded_messages_count(&self) -> usize {
        self.inner.messages.len()
    }

    /// Returns the message at the given index.
    #[frb(sync, type_64bit_int, positional)]
    pub fn message_at(&self, index: usize) -> Option<UiConversationMessage> {
        self.inner.messages.get(index).cloned()
    }

    /// Returns the lookup table mapping a message id to the index in the list.
    #[frb(sync, type_64bit_int, positional)]
    pub fn message_id_index(&self, message_id: UiConversationMessageId) -> Option<usize> {
        self.inner.message_ids_index.get(&message_id).copied()
    }
}

/// Provides access the the list of messages in a conversation.
///
/// Currently, only the last 1000 messages are loaded. This is subject to change ([#287]).
///
/// [#287]: https://github.com/phnx-im/infra/issues/287
#[frb(opaque)]
pub struct MessageListCubitBase {
    core: CubitCore<MessageListState>,
}

impl MessageListCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, conversation_id: ConversationId) -> Self {
        let store = user_cubit.core_user.clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::new();

        MessageListContext::new(store, core.state_tx().clone(), conversation_id.into())
            .spawn(store_notifications, core.cancellation_token().clone());

        Self { core }
    }

    // Cubit interface

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.core.is_closed()
    }

    pub fn close(&mut self) {
        self.core.close();
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> MessageListState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<MessageListState>) {
        self.core.stream(sink).await;
    }
}

/// Loads the initial state and listen to the changes a background task.
#[frb(ignore)]
#[derive(Clone)]
struct MessageListContext<S> {
    store: S,
    state_tx: watch::Sender<MessageListState>,
    conversation_id: ConversationId,
}

impl<S: Store + Send + Sync + 'static> MessageListContext<S> {
    fn new(
        store: S,
        state_tx: watch::Sender<MessageListState>,
        conversation_id: ConversationId,
    ) -> Self {
        Self {
            store,
            state_tx,
            conversation_id,
        }
    }

    fn spawn(
        self,
        store_notifications: impl Stream<Item = Arc<StoreNotification>> + Send + 'static,
        stop: CancellationToken,
    ) {
        spawn_from_sync(async move {
            self.load_and_emit_state().await;
            self.store_notifications_loop(store_notifications, stop)
                .await;
        });
    }

    async fn load_and_emit_state(&self) {
        const MAX_MESSAGES: usize = 1001;
        let messages = match self
            .store
            .messages(self.conversation_id, MAX_MESSAGES)
            .await
        {
            Ok(messages) => messages,
            Err(error) => {
                error!(
                    conversation_id =% self.conversation_id,
                    %error,
                    "Failed to load messages"
                );
                return;
            }
        };
        debug!(?messages, "MessageListCubit::load_and_emit_state");
        let include_first = messages.len() < MAX_MESSAGES;
        self.state_tx
            .send_modify(|state| state.rebuild_from_messages(messages, include_first));
    }

    async fn store_notifications_loop(
        &self,
        store_notifications: impl Stream<Item = Arc<StoreNotification>>,
        stop: CancellationToken,
    ) {
        let mut store_notifications = pin!(store_notifications);
        loop {
            let res = tokio::select! {
                _ = stop.cancelled() => return,
                notification = store_notifications.next() => notification,
            };
            match res {
                Some(notification) => {
                    self.process_store_notification(&notification).await;
                }
                None => return,
            }
        }
    }

    async fn process_store_notification(&self, notification: &StoreNotification) {
        if let Err(error) = self.try_process_store_notification(notification).await {
            error!(%error, "Failed to process store notification");
        }
    }

    async fn try_process_store_notification(
        &self,
        notification: &StoreNotification,
    ) -> anyhow::Result<()> {
        for item in notification.ops.iter() {
            if let (StoreEntityId::Message(message_id), StoreOperation::Add) = item {
                if let Some(message) = self.store.message(*message_id).await? {
                    if message.conversation_id() == self.conversation_id {
                        self.notify_neghbors_of_added_message(message);
                        self.load_and_emit_state().await;
                    }
                    return Ok(());
                };
            }
        }
        Ok(())
    }

    /// Send update notification to the neighbors of the added message.
    ///
    /// The neighbors are calculated from the list of loaded messages by looking up the position of
    /// the `message` in list by timestamp.
    fn notify_neghbors_of_added_message(&self, message: ConversationMessage) {
        let state = self.state_tx.borrow();
        let messages = &state.inner.messages;
        match messages.binary_search_by_key(&Some(message.timestamp()), |m| m.timestamp()) {
            Ok(_idx) => {
                warn!("Added message is already in the list");
            }
            Err(idx) => {
                let prev_message = idx.checked_sub(1).and_then(|idx| messages.get(idx));
                let next_message = messages.get(idx);
                let mut notification = StoreNotification::default();
                if let Some(message) = prev_message {
                    notification.ops.insert(
                        StoreEntityId::Message(message.id.into()),
                        StoreOperation::Update,
                    );
                }
                if let Some(message) = next_message {
                    notification.ops.insert(
                        StoreEntityId::Message(message.id.into()),
                        StoreOperation::Update,
                    );
                }
                self.store.notify(notification);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phnxcoreclient::{ContentMessage, ConversationMessageId, Message, MimiContent};
    use phnxtypes::time::TimeStamp;
    use uuid::Uuid;

    use super::*;

    fn new_test_message(sender: &str, timestamp_secs: i64) -> ConversationMessage {
        ConversationMessage::new_for_test(
            ConversationId::from(Uuid::from_u128(1)),
            ConversationMessageId::from_uuid(Uuid::from_u128(1)),
            TimeStamp::from(timestamp_secs * 1_000_000_000),
            Message::with_content(ContentMessage::new(
                sender.into(),
                true,
                MimiContent::simple_markdown_message(
                    "localhost".parse().unwrap(),
                    "some content".into(),
                ),
            )),
        )
    }

    #[test]
    fn test_rebuild_from_messages_flight_positions() {
        use UiFlightPosition::*;

        let messages = vec![
            new_test_message("alice", 0),
            new_test_message("alice", 1),
            new_test_message("alice", 2),
            // -- break due to sender
            new_test_message("bob", 3),
            new_test_message("bob", 4),
            new_test_message("bob", 5),
            // -- break due to time
            new_test_message("bob", 65),
            // -- break due to sender and time
            new_test_message("alice", 125),
            new_test_message("alice", 126),
        ];

        let mut state = MessageListState::default();
        let include_first = true;
        state.rebuild_from_messages(messages.clone(), include_first);

        let positions = state
            .inner
            .messages
            .iter()
            .map(|m| m.position)
            .collect::<Vec<_>>();
        assert_eq!(
            positions,
            [Start, Middle, End, Start, Middle, End, Unique, Start, End]
        );

        let mut state = MessageListState::default();
        let include_first = false;
        state.rebuild_from_messages(messages.clone(), include_first);

        let positions = state
            .inner
            .messages
            .iter()
            .map(|m| m.position)
            .collect::<Vec<_>>();
        assert_eq!(
            positions,
            [Middle, End, Start, Middle, End, Unique, Start, End]
        );
    }
}
