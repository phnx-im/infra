// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
};
use phnxcoreclient::{store::StoreNotification, ConversationId};
use phnxtypes::identifiers::SafeTryInto;
use tokio::{runtime::Handle, sync::watch, task::block_in_place};
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::StreamSink;
use crate::{
    util::{spawn_from_sync, Cubit, CubitCore},
    FLUTTER_RUST_BRIDGE_HANDLER,
};

use super::types::{UiConversationDetails, UiConversationType, UiUserProfile};
use super::user::user_cubit::UserCubitBase;
use super::{conversations::converation_into_ui_details, types::UiConversationMessageId};

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

    #[frb(sync, type_64bit_int)]
    pub fn message_id_from_rev_offset(&self, offset: usize) -> Option<UiConversationMessageId> {
        // TODO: This is a hack, but we need a sync version of this method. Can we do better?
        let _rt = FLUTTER_RUST_BRIDGE_HANDLER.async_runtime().0.enter();
        block_in_place(|| {
            Handle::current()
                .block_on(
                    self.store
                        .message_id_from_rev_offset(self.conversation_id, offset),
                )
                .map(From::from)
        })
    }

    #[frb(sync, type_64bit_int)]
    pub fn rev_offset_from_message_id(&self, message_id: UiConversationMessageId) -> Option<usize> {
        // TODO: This is a hack, but we need a sync version of this method. Can we do better?
        let _rt = FLUTTER_RUST_BRIDGE_HANDLER.async_runtime().0.enter();
        block_in_place(|| {
            Handle::current()
                .block_on(
                    self.store
                        .rev_offset_from_message_id(self.conversation_id, message_id.into()),
                )
                .map(From::from)
        })
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
        let conversation_id = StoreEntityId::Conversation(self.conversation_id);
        let conversation_changed = notification.ops.iter().any(|(id, op)| {
            id == &conversation_id && matches!(op, StoreOperation::Add | StoreOperation::Update)
        });
        if conversation_changed {
            self.load_and_emit_state().await;
        }
    }
}
