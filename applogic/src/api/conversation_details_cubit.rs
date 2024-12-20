// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::util::{spawn_from_sync, Cubit, CubitCore};
use crate::StreamSink;

use super::conversations::converation_into_ui_details;
use super::messages::{FetchedMessages, FetchedMessagesBroadcast, FetchedMessagesReceiver};
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
    core: CubitCore<ConversationDetailsState>,
    conversation_id: ConversationId,
    core_user: CoreUser,
    fetched_messages_tx: FetchedMessagesBroadcast,
}

impl ConversationDetailsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, conversation_id: ConversationId) -> Self {
        let core_user = user_cubit.core_user.clone();
        let core = CubitCore::new();

        ConversationDetailsContext::new(
            core_user.clone(),
            core.state_tx().clone(),
            conversation_id,
        )
        .spawn(
            user_cubit.subscribe_to_fetched_messages(),
            core.cancellation_token().clone(),
        );

        Self {
            core,
            conversation_id,
            core_user,
            fetched_messages_tx: user_cubit.fetched_messages_tx().clone(),
        }
    }

    // Cubit interface

    pub fn close(&mut self) {
        self.core.close();
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.core.is_closed()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> ConversationDetailsState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<ConversationDetailsState>) {
        self.core.stream(sink).await;
    }

    // Cubit methods

    pub async fn set_conversation_picture(&mut self, bytes: Option<Vec<u8>>) -> anyhow::Result<()> {
        self.core_user
            .set_conversation_picture(self.conversation_id, bytes.clone())
            .await?;
        self.fetched_messages_tx
            .send(FetchedMessages {
                changed_conversations: vec![self.conversation_id],
                ..Default::default()
            })
            .await;
        Ok(())
    }

    /// Load user profile of the conversation (only for non-group conversations)
    pub async fn load_conversation_user_profile(&self) -> anyhow::Result<Option<UiUserProfile>> {
        let conversation_type = self
            .core
            .borrow_state()
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
#[frb(ignore)]
struct ConversationDetailsContext {
    core_user: CoreUser,
    state_tx: watch::Sender<ConversationDetailsState>,
    conversation_id: ConversationId,
}

impl ConversationDetailsContext {
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

    fn spawn(self, fetched_messages_rx: FetchedMessagesReceiver, stop: CancellationToken) {
        spawn_from_sync(async move {
            self.load_and_emit_state().await;
            self.fetched_messages_listen_loop(fetched_messages_rx, stop)
                .await;
        });
    }

    async fn load_and_emit_state(&self) -> Option<()> {
        let details = self.load_conversation_details().await?;
        let members = self
            .members_of_conversation()
            .await
            .inspect_err(|error| error!(%error, "Failed fetching members"))
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
