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
use tracing::{debug, error};

use crate::{
    util::{spawn_from_sync, Cubit, CubitCore},
    StreamSink,
};

use super::{
    types::{UiConversationMessage, UiConversationMessageId},
    user::user_cubit::UserCubitBase,
};

#[frb(opaque)]
#[derive(Debug, Default, Clone)]
pub struct MessageListState {
    /// Copy-on-write inner ref to make the state cheaply clonable when emitting new state
    inner: Arc<MessageListStateInner>,
}

#[frb(ignore)]
#[derive(Debug, Default)]
struct MessageListStateInner {
    /// loaded messages (not all messages in the conversation)
    messages: Vec<UiConversationMessage>,
    /// lookup index from message id to index in `messages`
    message_ids_index: HashMap<UiConversationMessageId, usize>,
}

impl MessageListState {
    /// Rebuild the state from loaded messages
    ///
    /// The state is fully replaced. Note: This behavior will change when we will introduce loading
    /// of additional messages by paging.
    fn rebuild_from_messages(&mut self, new_messages: Vec<ConversationMessage>) {
        let inner = MessageListStateInner {
            message_ids_index: new_messages
                .iter()
                .enumerate()
                .map(|(index, message)| (message.id().into(), index))
                .collect(),
            messages: new_messages.into_iter().map(From::from).collect(),
        };
        self.inner = Arc::new(inner); // copy on write
    }

    #[frb(sync, getter, type_64bit_int)]
    pub fn loaded_messages_count(&self) -> usize {
        self.inner.messages.len()
    }

    #[frb(sync, type_64bit_int, positional)]
    pub fn message_at(&self, index: usize) -> Option<UiConversationMessage> {
        self.inner.messages.get(index).cloned()
    }

    #[frb(sync, type_64bit_int, positional)]
    pub fn message_id_index(&self, message_id: UiConversationMessageId) -> Option<usize> {
        self.inner.message_ids_index.get(&message_id).copied()
    }
}

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

/// Loads the intial state and listen to the changes
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
        const MAX_MESSAGES: usize = 1000;
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
        self.state_tx
            .send_modify(|state| state.rebuild_from_messages(messages));
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
                        self.load_and_emit_state().await;
                        return Ok(());
                    }
                };
            }
        }
        Ok(())
    }
}
