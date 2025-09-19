// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! List of chats feature

use std::sync::Arc;

use aircommon::identifiers::UserHandle;
use aircoreclient::{
    Chat,
    clients::CoreUser,
    store::{Store, StoreEntityId},
};
use aircoreclient::{ChatId, store::StoreNotification};
use flutter_rust_bridge::frb;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::{
    StreamSink,
    api::types::{UiChatMessage, UiMessageDraft, UiMessageDraftSource},
    util::{Cubit, CubitCore, spawn_from_sync},
};

use super::{
    types::{UiChatDetails, UiChatType, UiUserHandle},
    user_cubit::UserCubitBase,
};

/// Represents the state of the list of chat.
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ChatListState {
    pub chats: Vec<UiChatDetails>,
}

/// Provides access to the list of chat.
#[frb(opaque)]
pub struct ChatListCubitBase {
    core: CubitCore<ChatListState>,
    context: ChatListContext<CoreUser>,
}

impl ChatListCubitBase {
    /// Creates a new chat list cubit.
    ///
    /// Loads the list of chats in the background and listens to the changes in the
    /// chats.
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let store = user_cubit.core_user().clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::new();

        let context = ChatListContext::new(store, core.state_tx().clone());
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
    pub fn state(&self) -> ChatListState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<ChatListState>) {
        self.core.stream(sink).await;
    }

    // Cubit methods

    /// Creates a new 1:1 connection with the given user via a user handle.
    ///
    /// Returns `None` if the provided handle does not exist.
    pub async fn create_contact_chat(
        &self,
        handle: UiUserHandle,
    ) -> anyhow::Result<Option<ChatId>> {
        let handle = UserHandle::new(handle.plaintext)?;
        self.context.store.add_contact(handle).await
    }

    /// Creates a new group chat with the given name.
    ///
    /// After the chat is created, the current user is the only member of the group.
    pub async fn create_group_chat(&self, group_name: String) -> anyhow::Result<ChatId> {
        let id = self.context.store.create_chat(group_name, None).await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }
}

/// Loads the initial state and listen to the changes
#[frb(ignore)]
#[derive(Clone)]
struct ChatListContext<S> {
    store: S,
    state_tx: watch::Sender<ChatListState>,
}

impl<S> ChatListContext<S>
where
    S: Store + Send + Sync + 'static,
{
    fn new(store: S, state_tx: watch::Sender<ChatListState>) -> Self {
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
        let chats = chat_details(&self.store).await;
        debug!(?chats, "load_and_emit_state");
        self.state_tx.send_modify(|state| state.chats = chats);
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
        let any_chat_changed = notification.ops.iter().any(|(id, op)| {
            matches!(id, StoreEntityId::Chat(_) if !op.is_empty())
                || matches!(id, StoreEntityId::User(_) if !op.is_empty())
        });
        if any_chat_changed {
            // TODO(perf): This is a very coarse-grained approach. Optimally, we would only load
            // changed and new chats, and replace them individually in the `state`.
            self.load_and_emit_state().await;
        }
    }
}

async fn chat_details(store: &impl Store) -> Vec<UiChatDetails> {
    let chatss = store.chats().await.unwrap_or_default();
    let mut chat_details = Vec::with_capacity(chatss.len());
    for chat in chatss {
        let details = load_chat_details(store, chat).await;
        chat_details.push(details);
    }
    // Sort the chat first by last updated draft if any, then by last used timestamp in
    // descending order
    chat_details.sort_unstable_by(|a, b| {
        b.draft
            .as_ref()
            .filter(|draft| !draft.is_empty())
            .map(|draft| draft.updated_at)
            .cmp(
                &a.draft
                    .as_ref()
                    .filter(|draft| !draft.is_empty())
                    .map(|draft| draft.updated_at),
            )
            .then(b.last_used.cmp(&a.last_used))
    });
    chat_details
}

/// Loads additional details for a chat and converts it into a
/// [`UiChatDetails`]
pub(super) async fn load_chat_details(store: &impl Store, chat: Chat) -> UiChatDetails {
    let messages_count = store.messages_count(chat.id()).await.unwrap_or_default();
    let unread_messages = store
        .unread_messages_count(chat.id())
        .await
        .unwrap_or_default();
    let last_message = store
        .last_message(chat.id())
        .await
        .ok()
        .flatten()
        .map(From::from);
    let last_used = last_message
        .as_ref()
        .map(|m: &UiChatMessage| m.timestamp.clone())
        .unwrap_or_default();
    // default is UNIX_EPOCH

    let chat_type = UiChatType::load_from_chat_type(store, chat.chat_type).await;

    let draft = store
        .message_draft(chat.id)
        .await
        .unwrap_or_default()
        .map(|d| UiMessageDraft::from_draft(d, UiMessageDraftSource::System));

    UiChatDetails {
        id: chat.id,
        status: chat.status.into(),
        chat_type,
        last_used,
        attributes: chat.attributes.into(),
        messages_count,
        unread_messages,
        last_message,
        draft,
    }
}
