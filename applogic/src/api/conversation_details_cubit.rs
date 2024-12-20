// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{error, warn};

use crate::util::spawn_from_sync;
use crate::StreamSink;

use super::conversations::converation_into_ui_details;
use super::messages::{FetchedMessages, FetchedMessagesReceiver};
use super::types::{UiConversationDetails, UiConversationType, UiUserProfile};
use super::user::user_cubit::UserCubitBase;

#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ConversationDetailsState {
    pub conversation: Option<UiConversationDetails>,
    pub members: Vec<String>,
}

#[frb(opaque)]
pub struct ConversationDetailsCubitBase {
    conversation_id: ConversationId,
    state_tx: watch::Sender<ConversationDetailsState>,
    sinks_tx: mpsc::Sender<StreamSink<ConversationDetailsState>>,
    core_user: CoreUser,
    background_tasks_cancel: Option<DropGuard>,
}

impl ConversationDetailsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, conversation_id: ConversationId) -> Self {
        let core_user = user_cubit.core_user.clone();
        let cancel = CancellationToken::new();

        let (state_tx, state_rx) = watch::channel(ConversationDetailsState::default());
        let (sinks_tx, sinks_rx) = mpsc::channel(16);

        spawn_from_sync(emitter_loop(state_rx, sinks_rx, cancel.clone()));

        let task = BackgroundTaskContext::new(core_user.clone(), state_tx.clone(), conversation_id)
            .run(user_cubit.subscribe_to_fetched_messages(), cancel.clone());
        spawn_from_sync(task);

        Self {
            conversation_id,
            state_tx,
            sinks_tx,
            core_user,
            background_tasks_cancel: Some(cancel.drop_guard()),
        }
    }

    // Cubit interface

    pub fn close(&mut self) {
        self.background_tasks_cancel.take();
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.background_tasks_cancel.is_none()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> ConversationDetailsState {
        self.state_tx.borrow().clone()
    }

    pub async fn stream(&mut self, sink: StreamSink<ConversationDetailsState>) {
        if self.sinks_tx.send(sink).await.is_err() {
            self.close();
        }
    }

    // Cubit methods

    pub async fn set_conversation_picture(&mut self, bytes: Option<Vec<u8>>) -> anyhow::Result<()> {
        self.core_user
            .set_conversation_picture(self.conversation_id, bytes.clone())
            .await?;
        self.state_tx
            .send_if_modified(|state| match state.conversation.as_mut() {
                Some(conversation) => {
                    conversation.attributes.conversation_picture_option = bytes;
                    true
                }
                None => false,
            });
        Ok(())
    }

    /// Load user profile of the conversation (only for non-group conversations)
    pub async fn load_conversation_user_profile(&self) -> anyhow::Result<Option<UiUserProfile>> {
        let conversation_type = self
            .state_tx
            .borrow()
            .conversation
            .as_ref()
            .map(|c| c.conversation_type.clone());
        match conversation_type {
            Some(
                UiConversationType::UnconfirmedConnection(username)
                | UiConversationType::Connection(username),
            ) => {
                let qualified_username = SafeTryInto::try_into(username)?;
                let profile = self.core_user.user_profile(&qualified_username).await?;
                Ok(profile.map(|profile| UiUserProfile::from_profile(&profile)))
            }
            Some(UiConversationType::Group) | None => Ok(None),
        }
    }
}

/// Loads the intial state and listen to the changes
struct BackgroundTaskContext {
    core_user: CoreUser,
    state_tx: watch::Sender<ConversationDetailsState>,
    conversation_id: ConversationId,
}

impl BackgroundTaskContext {
    fn new(
        core_user: CoreUser,
        state_tx: watch::Sender<ConversationDetailsState>,
        conversation_id: ConversationId,
    ) -> Self {
        Self {
            core_user,
            state_tx,
            conversation_id,
        }
    }

    async fn run(self, fetched_messages_rx: FetchedMessagesReceiver, stop: CancellationToken) {
        self.load_conversation_and_listen(fetched_messages_rx, stop)
            .await;
    }

    async fn load_conversation_and_listen(
        self,
        fetched_messages_rx: FetchedMessagesReceiver,
        stop: CancellationToken,
    ) {
        self.load_and_emit_state().await;
        self.fetched_messages_listen_loop(fetched_messages_rx, stop)
            .await;
    }

    async fn load_and_emit_state(&self) -> Option<()> {
        let details = self.load_conversation_details().await?;
        let members = self
            .members_of_conversation()
            .await
            .inspect_err(|error| error!(%error, "Error when fetching members"))
            .unwrap_or_default();
        let new_state = ConversationDetailsState {
            conversation: Some(details),
            members,
        };
        self.state_tx.send(new_state).ok()
    }

    async fn load_conversation_details(&self) -> Option<UiConversationDetails> {
        let conversation = self.core_user.conversation(&self.conversation_id).await?;
        Some(converation_into_ui_details(&self.core_user, conversation).await)
    }

    async fn members_of_conversation(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .core_user
            .conversation_participants(self.conversation_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.to_string())
            .collect())
    }

    /// Returns only when `stop` is cancelled
    async fn fetched_messages_listen_loop(
        self,
        mut fetched_messages_rx: FetchedMessagesReceiver,
        stop: CancellationToken,
    ) {
        loop {
            let res = tokio::select! {
                res = fetched_messages_rx.recv() => res,
                _ = stop.cancelled() => return,
            };
            match res {
                Ok(fetched_messages) => self.handle_fetched_messages(&fetched_messages).await,
                Err(broadcast::error::RecvError::Closed) => return,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(n, "fetched messages lagged");
                }
            }
        }
    }

    async fn handle_fetched_messages(
        &self,
        FetchedMessages {
            new_conversations,
            changed_conversations,
            new_messages: _,
            notifications_content: _,
        }: &FetchedMessages,
    ) {
        if changed_conversations.contains(&self.conversation_id)
            || new_conversations.contains(&self.conversation_id)
        {
            self.load_and_emit_state().await;
        }
    }
}

async fn emitter_loop(
    mut state_rx: watch::Receiver<ConversationDetailsState>,
    mut sinks_rx: mpsc::Receiver<StreamSink<ConversationDetailsState>>,
    stop: CancellationToken,
) {
    let mut sinks = Vec::new();
    loop {
        tokio::select! {
            sink = sinks_rx.recv() => {
                let Some(sink) = sink else { return };
                sinks.push(sink);
            },
            changed = state_rx.changed() => {
                if changed.is_err() {
                    return;
                };
                let state = state_rx.borrow().clone();
                sinks.retain(|sink| sink.add(state.clone()).is_ok());
            },
            _ = stop.cancelled() => {
                return;
            }
        }
    }
}
