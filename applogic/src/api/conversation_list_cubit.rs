// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{pin::pin, sync::Arc};

use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
    Conversation,
};
use phnxcoreclient::{store::StoreNotification, ConversationId};
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::util::{spawn_from_sync, Cubit, CubitCore};
use crate::StreamSink;

use super::{
    types::{UiConversation, UiConversationDetails, UiConversationMessage},
    user_cubit::UserCubitBase,
};

#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ConversationListState {
    pub conversations: Vec<UiConversationDetails>,
}

#[frb(opaque)]
pub struct ConversationListCubitBase {
    core: CubitCore<ConversationListState>,
    context: ConversationListContext<CoreUser>,
}

impl ConversationListCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let store = user_cubit.core_user.clone();
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

    pub async fn create_connection(&self, user_name: String) -> anyhow::Result<ConversationId> {
        let id = self.context.store.add_contact(user_name).await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }

    pub async fn create_conversation(&self, group_name: String) -> anyhow::Result<ConversationId> {
        let id = self
            .context
            .store
            .create_conversation(&group_name, None)
            .await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }
}

/// Loads the intial state and listen to the changes
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
        let conversations = conversation_details(&self.store).await;
        debug!(?conversations, "load_and_emit_state");
        self.state_tx
            .send_modify(|state| state.conversations = conversations);
    }

    async fn store_notifications_loop(
        self,
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
        let any_conversation_changed = notification.ops.iter().any(|(id, op)| {
            matches!(
                (id, op),
                (StoreEntityId::Conversation(_), StoreOperation::Add)
                    | (StoreEntityId::Conversation(_), StoreOperation::Remove)
                    | (StoreEntityId::Conversation(_), StoreOperation::Update)
            )
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
        .map(|m| m.into());
    let last_used = last_message
        .as_ref()
        .map(|m: &UiConversationMessage| m.timestamp.clone())
        .unwrap_or_default();
    // default is UNIX_EPOCH

    let conversation = UiConversation::from(conversation);
    UiConversationDetails {
        id: conversation.id,
        group_id: conversation.group_id,
        status: conversation.status,
        conversation_type: conversation.conversation_type,
        last_used,
        attributes: conversation.attributes,
        messages_count: TryInto::try_into(messages_count).expect("usize overflow"),
        unread_messages: TryInto::try_into(unread_messages).expect("usize overflow"),
        last_message,
    }
}
