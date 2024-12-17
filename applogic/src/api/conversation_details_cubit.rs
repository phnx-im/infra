// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use flutter_rust_bridge::frb;
use log::{error, warn};
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::util::{spawn_from_sync, SharedCubitSinks};
use crate::StreamSink;

use super::conversations::converation_into_ui_details;
use super::messages::{FetchedMessages, FetchedMessagesReceiver};
use super::types::{UiConversationDetails, UiConversationType, UiUserProfile};
use super::user::user_cubit::UserCubitBase;

type State = Arc<RwLock<ConversationDetailsState>>;

#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ConversationDetailsState {
    pub conversation: Option<UiConversationDetails>,
    pub members: Vec<String>,
}

#[frb(opaque)]
pub struct ConversationDetailsCubitBase {
    shared: Shared,
    _background_task_cancel: DropGuard,
}

impl ConversationDetailsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, conversation_id: ConversationId) -> Self {
        let core_user = user_cubit.core_user.clone();
        let shared = Shared::new(core_user.clone(), conversation_id);
        let cancel = CancellationToken::new();

        // background task to load the conversation details and listen to changes
        spawn_from_sync({
            let shared = shared.clone();
            let cancel = cancel.clone();
            let rx = user_cubit.subscribe_to_fetched_messages();
            async move {
                shared.load_ui_conversation_details().await;
                shared.process_fetched_messages(rx, cancel).await;
            }
        });

        Self {
            shared,
            _background_task_cancel: cancel.drop_guard(),
        }
    }

    // Cubit interface

    pub fn close(&mut self) {
        self.shared.sinks.close();
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        // Note: don't lock too long, this is the UI thread
        self.is_closed()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> ConversationDetailsState {
        // Note: don't lock too long, this is the UI thread
        self.shared.state.blocking_read().clone()
    }

    pub async fn stream(&mut self, sink: StreamSink<ConversationDetailsState>) {
        self.shared.sinks.push(sink).await;
    }

    // Cubit methods

    pub async fn set_conversation_picture(&mut self, bytes: Option<Vec<u8>>) -> anyhow::Result<()> {
        let conversation_id = self.shared.conversation_id;
        self.shared
            .core_user
            .set_conversation_picture(conversation_id, bytes.clone())
            .await?;
        self.shared
            .emit(|state| {
                let conversation = state.conversation.as_mut()?;
                conversation.attributes.conversation_picture_option = bytes;
                Some(())
            })
            .await;
        Ok(())
    }

    /// User profile of the conversation (only for non-group conversations)
    pub async fn load_conversation_user_profile(&self) -> anyhow::Result<Option<UiUserProfile>> {
        let conversation_type = self
            .shared
            .state
            .read()
            .await
            .conversation
            .as_ref()
            .map(|c| c.conversation_type.clone());
        match conversation_type {
            Some(
                UiConversationType::UnconfirmedConnection(username)
                | UiConversationType::Connection(username),
            ) => {
                let qualified_username = SafeTryInto::try_into(username)?;
                let profile = self
                    .shared
                    .core_user
                    .user_profile(&qualified_username)
                    .await?;
                Ok(profile.map(|profile| UiUserProfile::from_profile(&profile)))
            }
            Some(UiConversationType::Group) | None => Ok(None),
        }
    }
}

/// Shared state between the UI thread and background tasks of the conversation details cubit
#[frb(ignore)]
#[derive(Clone)]
struct Shared {
    conversation_id: ConversationId,
    state: Arc<RwLock<ConversationDetailsState>>,
    sinks: SharedCubitSinks<ConversationDetailsState>,
    core_user: CoreUser,
}

impl Shared {
    fn new(core_user: CoreUser, conversation_id: ConversationId) -> Self {
        Self {
            conversation_id,
            state: State::default(),
            sinks: SharedCubitSinks::default(),
            core_user,
        }
    }

    /// Updates the state and emits it to the sinks if the `update` function returned `Some`
    async fn emit(&self, update: impl FnOnce(&mut ConversationDetailsState) -> Option<()>) {
        let new_state = {
            let mut state = self.state.write().await;
            if update(&mut state).is_none() {
                return;
            }
            state.clone()
        };
        self.sinks.emit(new_state).await;
    }

    /// Loads and emits the conversation details
    async fn load_ui_conversation_details(&self) {
        let Some(details) = self.load_conversation_details().await else {
            return;
        };
        let members = self
            .members_of_conversation()
            .await
            .inspect_err(|error| error!("Error when fetching members: {error}"))
            .unwrap_or_default();
        let new_state = ConversationDetailsState {
            conversation: Some(details),
            members,
        };
        self.emit(|state| {
            *state = new_state;
            Some(())
        })
        .await;
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
    async fn process_fetched_messages(
        &self,
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
                    warn!("fetched messages lagged {n} messages");
                }
            }
        }
    }

    async fn handle_fetched_messages(
        &self,
        FetchedMessages {
            new_conversations: _,
            changed_conversations,
            new_messages: _,
            notifications_content: _,
        }: &FetchedMessages,
    ) {
        if changed_conversations
            .iter()
            .any(|&id| id == self.conversation_id)
        {
            self.load_ui_conversation_details().await;
        }
    }
}
