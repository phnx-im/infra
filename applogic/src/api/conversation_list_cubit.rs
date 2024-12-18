// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use log::{error, info};
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;

use crate::api::user::User;
use crate::util::{spawn_from_sync, Cubit, CubitCore};
use crate::StreamSink;

use super::messages::{FetchedMessages, FetchedMessagesReceiver};
use super::types::UiConversationDetails;
use super::user::user_cubit::UserCubitBase;

#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ConversationListState {
    pub conversations: Vec<UiConversationDetails>,
}

#[frb(opaque)]
pub struct ConversationListCubitBase {
    core: CubitCore<ConversationListState>,
    context: ConversationListContext,
}

impl ConversationListCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        info!("ConversationListCubitBase::new");

        let core_user = user_cubit.core_user.clone();
        let core = CubitCore::new();

        let context = ConversationListContext::new(core_user.clone(), core.state_tx().clone());
        context.clone().spawn(
            user_cubit.subscribe_to_fetched_messages(),
            core.cancellation_token().clone(),
        );

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
        let id = self.context.core_user.add_contact(user_name).await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }

    pub async fn create_conversation(&self, group_name: String) -> anyhow::Result<ConversationId> {
        let id = self
            .context
            .core_user
            .create_conversation(&group_name, None)
            .await?;
        self.context.load_and_emit_state().await;
        Ok(id)
    }
}

/// Loads the intial state and listen to the changes
#[frb(ignore)]
#[derive(Clone)]
struct ConversationListContext {
    core_user: CoreUser,
    state_tx: watch::Sender<ConversationListState>,
}

impl ConversationListContext {
    fn new(core_user: CoreUser, state_tx: watch::Sender<ConversationListState>) -> Self {
        Self {
            core_user,
            state_tx,
        }
    }

    fn spawn(self, fetched_messages_rx: FetchedMessagesReceiver, stop: CancellationToken) {
        spawn_from_sync(async move {
            self.load_and_emit_state().await;
            self.fetched_messages_listen_loop(fetched_messages_rx, stop)
                .await;
        });
    }

    async fn load_and_emit_state(&self) {
        let user = User::with_empty_state(self.core_user.clone());
        let conversations = user.get_conversation_details().await;
        self.state_tx
            .send_modify(|state| state.conversations = conversations);
    }

    async fn fetched_messages_listen_loop(
        self,
        mut rx: FetchedMessagesReceiver,
        stop: CancellationToken,
    ) {
        loop {
            let res = tokio::select! {
                _ = stop.cancelled() => return,
                res = rx.recv() => res,
            };
            match res {
                Ok(fetched_messages) => {
                    self.process_fetches_messages(&fetched_messages).await;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    error!("Failed to fetch messages; lagging {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
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
        // TODO(perf): This is a very coarse-grained approach. Optimally, we would only load
        // changed and new conversations, and replace them individually in the `state`.
        if new_conversations.is_empty() && changed_conversations.is_empty() {
            return;
        }
        self.load_and_emit_state().await;
    }
}
