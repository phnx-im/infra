// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use flutter_rust_bridge::frb;
use log::error;
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::util::{spawn_from_sync, SharedCubitSinks};
use crate::StreamSink;

use super::messages::FetchedMessages;
use super::types::UiConversationDetails;
use super::user::user_cubit::UserCubitBase;
use super::user::User;

type State = Arc<RwLock<ConversationListState>>;

/// State of the [`ConversationListCubitBase`]
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversationListState {
    pub conversations: Vec<UiConversationDetails>,
}

/// Cubit that provides access to the conversation list
#[frb(opaque)]
pub struct ConversationListCubitBase {
    shared: Shared,
    _cancel: DropGuard,
}

impl ConversationListCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let shared = Shared::new(user_cubit.core_user.clone());
        let cancel = CancellationToken::new();

        /// task: listens to the fetched messages and updates the state
        let mut rx = user_cubit.subscribe_to_fetched_messages();
        let inner_shared = shared.clone();
        let inner_cancel = cancel.clone();
        spawn_from_sync(async move {
            // initial fetch
            inner_shared.refetch_conversations().await;

            loop {
                let res = tokio::select! {
                    _ = inner_cancel.cancelled() => return,
                    res = rx.recv() => res,
                };
                match res {
                    Ok(fetched_messages) => {
                        inner_shared
                            .process_fetches_messages(&fetched_messages)
                            .await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        error!("Failed to fetch messages; lagging {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });

        Self {
            shared,
            _cancel: cancel.drop_guard(),
        }
    }

    // Cubit inteface

    pub async fn close(&mut self) {
        self.shared.sinks.close();
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        // Note: don't lock too long, this is the UI thread
        self.is_closed()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> ConversationListState {
        // Note: don't lock too long, this is the UI thread
        self.shared.state.blocking_read().clone()
    }

    pub async fn stream(&mut self, sink: StreamSink<ConversationListState>) {
        self.shared.sinks.push(sink).await;
    }

    // Cubit methods

    pub async fn create_connection(&self, user_name: String) -> anyhow::Result<ConversationId> {
        let id = self.shared.core_user.add_contact(user_name).await?;
        self.shared.refetch_conversations().await;
        Ok(id)
    }

    pub async fn create_conversation(&self, group_name: String) -> anyhow::Result<ConversationId> {
        let id = self
            .shared
            .core_user
            .create_conversation(&group_name, None)
            .await?;
        self.shared.refetch_conversations().await;
        Ok(id)
    }
}

/// Shared state of the [`ConversationListCubitBase`] betwee the UI and the background task.
#[frb(ignore)]
#[derive(Clone)]
struct Shared {
    state: State,
    sinks: SharedCubitSinks<ConversationListState>,
    core_user: CoreUser,
}

impl Shared {
    fn new(core_user: CoreUser) -> Self {
        Self {
            state: State::default(),
            sinks: SharedCubitSinks::default(),
            core_user,
        }
    }

    async fn process_fetches_messages(
        &self,
        FetchedMessages {
            new_conversations,
            changed_conversations,
            new_messages: _,
            notifications_content: _,
        }: &FetchedMessages,
    ) {
        if new_conversations.is_empty() && changed_conversations.is_empty() {
            return;
        }
        self.refetch_conversations().await;
    }

    // TODO(perf): This is a very coarse-grained approach. Optimally, we would only load
    // changed and new conversations, and replace them individually in the `state`.
    async fn refetch_conversations(&self) {
        let instant = std::time::Instant::now();
        let mut new_state = self.state.read().await.clone();
        let user = User::with_empty_state(self.core_user.clone());
        new_state.conversations = user.get_conversation_details().await;
        log::info!(
            "Refetching conversations: elapsed = {:?}",
            instant.elapsed()
        );
        self.emit(new_state).await;
    }

    async fn emit(&self, new_state: ConversationListState) {
        *self.state.write().await = new_state.clone();
        self.sinks.emit(new_state).await;
    }
}
