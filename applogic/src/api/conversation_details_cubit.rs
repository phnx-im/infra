// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use flutter_rust_bridge::frb;
use phnxcoreclient::{clients::CoreUser, store::Store, MimiContent};
use phnxcoreclient::{store::StoreNotification, ConversationId};
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::util::{spawn_from_sync, Cubit, CubitCore};
use crate::StreamSink;

use super::conversations::converation_into_ui_details;
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
    store: CoreUser,
}

impl ConversationDetailsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, conversation_id: ConversationId) -> Self {
        let store = user_cubit.core_user.clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::new();

        ConversationDetailsContext::new(store.clone(), core.state_tx().clone(), conversation_id)
            .spawn(store_notifications, core.cancellation_token().clone());

        Self {
            core,
            conversation_id,
            store,
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
        Store::set_conversation_picture(&self.store, self.conversation_id, bytes.clone()).await
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
                let profile = self.store.user_profile(&qualified_username).await?;
                Ok(profile.map(|profile| UiUserProfile::from_profile(&profile)))
            }
            Some(UiConversationType::Group) | None => Ok(None),
        }
    }

    pub async fn send_message(&self, message_text: String) -> anyhow::Result<()> {
        let domain = self.store.user_name().domain();
        let content = MimiContent::simple_markdown_message(domain, message_text);
        self.store
            .send_message(self.conversation_id, content)
            .await
            .inspect_err(|error| error!(%error, "Failed to send message"))?;
        Ok(())
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
        let details = self.load_conversation_details().await;
        let members = if details.is_some() {
            self.members_of_conversation()
                .await
                .inspect_err(|error| error!(%error, "Failed fetching members"))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let new_state = ConversationDetailsState {
            conversation: details,
            members,
        };
        let _ = self.state_tx.send(new_state);
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
    async fn store_notifications_loop(
        self,
        store_notifications: impl Stream<Item = Arc<StoreNotification>>,
        stop: CancellationToken,
    ) {
        let mut store_notifications = std::pin::pin!(store_notifications);
        loop {
            let res = tokio::select! {
                notification = store_notifications.next() => notification,
                _ = stop.cancelled() => return,
            };
            match res {
                Some(notification) => self.handle_store_notification(&notification).await,
                None => return,
            }
        }
    }

    async fn handle_store_notification(&self, notification: &StoreNotification) {
        if notification.ops.contains_key(&self.conversation_id.into()) {
            self.load_and_emit_state().await;
        }
    }
}
