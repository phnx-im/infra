// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{pin::pin, sync::Arc};

use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreNotification, StoreOperation},
    ConversationMessageId,
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

#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct MessageState {
    pub message: Option<UiConversationMessage>,
}

#[frb(opaque)]
pub struct MessageCubitBase {
    core: CubitCore<MessageState>,
    store: CoreUser,
}

impl MessageCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, message_id: UiConversationMessageId) -> Self {
        let message_id = message_id.into();

        let store = user_cubit.core_user.clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::new();

        MessageContext::new(store.clone(), core.state_tx().clone(), message_id)
            .spawn(store_notifications, core.cancellation_token().clone());

        Self { core, store }
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
    pub fn state(&self) -> MessageState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<MessageState>) {
        self.core.stream(sink).await;
    }

    // Cubit methods

    pub async fn mark_as_read(&self) -> anyhow::Result<()> {
        let Some((conversation_id, timestamp)) = self
            .core
            .state_tx()
            .borrow()
            .message
            .as_ref()
            .filter(|message| !message.is_read)
            .and_then(|message| {
                let timestamp: DateTime<Utc> = message.timestamp.parse().ok()?;
                Some((message.conversation_id, timestamp))
            })
        else {
            return Ok(());
        };
        debug!(%conversation_id, %timestamp, "Marking conversation as read");
        self.store
            .mark_conversation_as_read([(conversation_id, timestamp)])
            .await?;
        Ok(())
    }
}

/// Loads the intial state and listen to the changes
#[frb(ignore)]
#[derive(Clone)]
struct MessageContext<S> {
    store: S,
    state_tx: watch::Sender<MessageState>,
    message_id: ConversationMessageId,
}

impl<S: Store + Send + Sync + 'static> MessageContext<S> {
    fn new(
        store: S,
        state_tx: watch::Sender<MessageState>,
        message_id: ConversationMessageId,
    ) -> Self {
        Self {
            store,
            state_tx,
            message_id,
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
        let conversation_message = self.store.message_with_neighbors(self.message_id).await;
        tracing::info!(?conversation_message, "load_and_emit_state");
        match conversation_message {
            Ok(cm) => {
                self.state_tx
                    .send_modify(|state| state.message = cm.map(From::from));
            }
            Err(error) => {
                error!(?error, "loading message failed");
            }
        }
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
        match notification.ops.get(&self.message_id.into()) {
            Some(StoreOperation::Add | StoreOperation::Update) => self.load_and_emit_state().await,
            Some(StoreOperation::Remove) => {
                self.state_tx.send_modify(|state| state.message = None);
            }
            None => {
                // reload on added message when there is no next neighbor
                // TODO: We could better short-circuit this logic, if we knew the conversation id
                // of the added message.
                let has_next_neighbor = self
                    .state_tx
                    .borrow()
                    .message
                    .as_ref()
                    .map(|message| message.neighbors.next.is_some())
                    .unwrap_or(true);
                let message_id = self.message_id.to_uuid();
                tracing::info!(has_next_neighbor, %message_id, ?notification, "has_next_neighbor");
                if !has_next_neighbor {
                    for item in notification.ops.iter() {
                        // TODO: There is a bug, where Update of the message overrides the Add
                        // operation. To mititage this, we check also for the Update operation.
                        if let (
                            StoreEntityId::Message(_),
                            StoreOperation::Add | StoreOperation::Update,
                        ) = item
                        {
                            self.load_and_emit_state().await;
                            return;
                        }
                    }
                }
            }
        }
    }
}
