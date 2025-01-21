// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{pin::pin, sync::Arc};

use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
};
use phnxcoreclient::{store::StoreNotification, ConversationId};
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::util::{spawn_from_sync, Cubit, CubitCore};
use crate::StreamSink;

use super::user::user_cubit::UserCubitBase;
use super::{conversations::ConversationsExt, types::UiConversationDetails};

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

    /// Creates a new 1:1 conenction with the given user.
    ///
    /// `user_name` is the fully qualified user name of the contact.
    pub async fn create_connection(&self, user_name: String) -> anyhow::Result<ConversationId> {
        let id = self.context.store.add_contact(user_name.parse()?).await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }

    /// Creates a new group conversation with the given name.
    ///
    /// After the conversation is created, the current user is the only member of the group.
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
        let conversations = self.store.conversation_details().await;
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
