// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! List of conversations feature

use std::sync::Arc;

use flutter_rust_bridge::frb;
use phnxcommon::identifiers::UserHandle;
use phnxcoreclient::{
    Conversation,
    clients::CoreUser,
    store::{Store, StoreEntityId},
};
use phnxcoreclient::{ConversationId, store::StoreNotification};
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::StreamSink;
use crate::util::{Cubit, CubitCore, spawn_from_sync};

use super::{
    types::{UiConversationDetails, UiConversationMessage, UiConversationType, UiUserHandle},
    user_cubit::UserCubitBase,
};

/// Represents the state of the list of conversations.
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ConversationListState {
    pub conversations: Vec<UiConversationDetails>,
}

/// Provides access to the list of conversations.
#[frb(opaque)]
pub struct ConversationListCubitBase {
    core: CubitCore<ConversationListState>,
    context: ConversationListContext<CoreUser>,
}

impl ConversationListCubitBase {
    /// Creates a new conversation list cubit.
    ///
    /// Loads the list of conversations in the background and listens to the changes in the
    /// conversations.
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let store = user_cubit.core_user().clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::new();

        let context = ConversationListContext::new(store, core.state_tx().clone());
        context
            .clone()
            .spawn(store_notifications, core.cancellation_token().clone());

        Self { core, context }
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
    pub fn state(&self) -> ConversationListState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<ConversationListState>) {
        self.core.stream(sink).await;
    }

    // Cubit methods

    /// Creates a new 1:1 connection with the given user via a user handle.
    ///
    /// Returns `None` if the provided handle does not exist.
    pub async fn create_connection(
        &self,
        handle: UiUserHandle,
    ) -> anyhow::Result<Option<ConversationId>> {
        let handle = UserHandle::new(handle.plaintext)?;
        self.context.store.add_contact(handle).await
    }

    /// Creates a new group conversation with the given name.
    ///
    /// After the conversation is created, the current user is the only member of the group.
    pub async fn create_conversation(&self, group_name: String) -> anyhow::Result<ConversationId> {
        let id = self
            .context
            .store
            .create_conversation(group_name, None)
            .await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }
}

/// Loads the initial state and listen to the changes
#[frb(ignore)]
#[derive(Clone)]
struct ConversationListContext<S> {
    store: S,
    state_tx: watch::Sender<ConversationListState>,
}

impl<S> ConversationListContext<S>
where
    S: Store + Send + Sync + 'static,
{
    fn new(store: S, state_tx: watch::Sender<ConversationListState>) -> Self {
        Self { store, state_tx }
    }

    fn spawn(
        self,
        store_notifications: impl Stream<Item = Arc<StoreNotification>> + Send + Unpin + 'static,
        stop: CancellationToken,
    ) {
        spawn_from_sync(async move {
            self.load_and_emit_state().await;
            self.store_notifications_loop(store_notifications, stop)
                .await;
        });
    }

    async fn load_and_emit_state(&self) {
        let conversations = conversation_details(&self.store).await;
        debug!(?conversations, "load_and_emit_state");
        self.state_tx
            .send_modify(|state| state.conversations = conversations);
    }

    async fn store_notifications_loop(
        self,
        mut store_notifications: impl Stream<Item = Arc<StoreNotification>> + Unpin,
        stop: CancellationToken,
    ) {
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
        let any_conversation_changed = notification.ops.iter().any(|(id, op)| {
            matches!(id, StoreEntityId::Conversation(_) if !op.is_empty())
                || matches!(id, StoreEntityId::User(_) if !op.is_empty())
        });
        if any_conversation_changed {
            // TODO(perf): This is a very coarse-grained approach. Optimally, we would only load
            // changed and new conversations, and replace them individually in the `state`.
            self.load_and_emit_state().await;
        }
    }
}

async fn conversation_details(store: &impl Store) -> Vec<UiConversationDetails> {
    let conversations = store.conversations().await.unwrap_or_default();
    let mut conversation_details = Vec::with_capacity(conversations.len());
    for conversation in conversations {
        let details = converation_into_ui_details(store, conversation).await;
        conversation_details.push(details);
    }
    // Sort the conversations by last used timestamp in descending order
    conversation_details.sort_unstable_by(|a, b| b.last_used.cmp(&a.last_used));
    conversation_details
}

/// Loads additional details for a conversation and converts it into a
/// [`UiConversationDetails`]
pub(super) async fn converation_into_ui_details(
    store: &impl Store,
    conversation: Conversation,
) -> UiConversationDetails {
    let messages_count = store
        .messages_count(conversation.id())
        .await
        .unwrap_or_default();
    let unread_messages = store
        .unread_messages_count(conversation.id())
        .await
        .unwrap_or_default();
    let last_message = store
        .last_message(conversation.id())
        .await
        .ok()
        .flatten()
        .map(UiConversationMessage::from_simple);
    let last_used = last_message
        .as_ref()
        .map(|m: &UiConversationMessage| m.timestamp.clone())
        .unwrap_or_default();
    // default is UNIX_EPOCH

    let conversation_type =
        UiConversationType::load_from_conversation_type(store, conversation.conversation_type)
            .await;

    UiConversationDetails {
        id: conversation.id,
        status: conversation.status.into(),
        conversation_type,
        last_used,
        attributes: conversation.attributes.into(),
        messages_count,
        unread_messages,
        last_message,
    }
}
