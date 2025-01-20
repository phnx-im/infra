// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{pin::pin, sync::Arc};

use flutter_rust_bridge::frb;
use phnxcoreclient::{
    store::{Store, StoreNotification, StoreOperation, StoreResult},
    ConversationMessageId,
};
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::{
    api::types::UiFlightPosition,
    util::{spawn_from_sync, Cubit, CubitCore},
    StreamSink,
};

use super::{types::UiConversationMessage, user_cubit::UserCubitBase};

/// State of a single message in a conversation.
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MessageState {
    pub message: UiConversationMessage,
}

/// Provides access to a single message in a conversation.
///
/// Listens to changes to the message and reloads it. On reload, also the previous and next
/// messages in the conversation timeline are loaded to calculate the flight position of this
/// message.
#[frb(opaque)]
pub struct MessageCubitBase {
    core: CubitCore<MessageState>,
}

impl MessageCubitBase {
    /// Creates a new message cubit.
    ///
    /// Note that the loaded message is immediately provided via `initial_state`.
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, initial_state: MessageState) -> Self {
        let message_id = initial_state.message.id.into();

        let store = user_cubit.core_user.clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::with_initial_state(initial_state);

        MessageContext::new(store, core.state_tx().clone(), message_id)
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
    pub fn state(&self) -> MessageState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<MessageState>) {
        self.core.stream(sink).await;
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
            self.store_notifications_loop(store_notifications, stop)
                .await;
        });
    }

    async fn load_and_emit_state(&self) {
        let conversation_message = self.store.message(self.message_id).await;

        debug!(?conversation_message, "load_and_emit_state");
        match conversation_message {
            Ok(Some(message)) => {
                let mut message = UiConversationMessage::from(message);
                message.position = calculate_flight_position(&self.store, &message)
                    .await
                    .inspect_err(|error| error!(?error, "Failed to calculate flight position"))
                    .unwrap_or(UiFlightPosition::Unique);
                self.state_tx.send_modify(|state| state.message = message);
            }
            Ok(None) => {}
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
            Some(StoreOperation::Remove) | None => {}
        }
    }
}

/// Calculate the flight position of a message by loading its previous and next messages.
async fn calculate_flight_position(
    store: &impl Store,
    message: &UiConversationMessage,
) -> StoreResult<UiFlightPosition> {
    let id = message.id.into();
    let prev_message = store.prev_message(id).await?.map(From::from);
    let next_message = store.next_message(id).await?.map(From::from);
    Ok(UiFlightPosition::calculate(
        message,
        prev_message.as_ref(),
        next_message.as_ref(),
    ))
}
