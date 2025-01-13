// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ops::ControlFlow, sync::Arc, time::Duration};

use chrono::{DateTime, SubsecRound, Utc};
use flutter_rust_bridge::frb;
use phnxcoreclient::{clients::CoreUser, store::Store, MimiContent};
use phnxcoreclient::{store::StoreNotification, ConversationId};
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::util::{spawn_from_sync, Cubit, CubitCore};
use crate::StreamSink;

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
    context: ConversationDetailsContext,
    core: CubitCore<ConversationDetailsState>,
}

impl ConversationDetailsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase, conversation_id: ConversationId) -> Self {
        let store = user_cubit.core_user.clone();
        let store_notifications = store.subscribe();

        let core = CubitCore::new();

        let context = ConversationDetailsContext::new(
            store.clone(),
            core.state_tx().clone(),
            conversation_id,
        );
        context
            .clone()
            .spawn(store_notifications, core.cancellation_token().clone());

        Self { context, core }
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
        Store::set_conversation_picture(
            &self.context.store,
            self.context.conversation_id,
            bytes.clone(),
        )
        .await
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
                let profile = self.context.store.user_profile(&qualified_username).await?;
                Ok(profile.map(|profile| UiUserProfile::from_profile(&profile)))
            }
            Some(UiConversationType::Group) | None => Ok(None),
        }
    }

    pub async fn send_message(&self, message_text: String) -> anyhow::Result<()> {
        let domain = self.context.store.user_name().domain();
        let content = MimiContent::simple_markdown_message(domain, message_text);
        self.context
            .store
            .send_message(self.context.conversation_id, content)
            .await
            .inspect_err(|error| error!(%error, "Failed to send message"))?;
        Ok(())
    }

    /// Marks the conversation as read until the given message id (including).
    pub async fn mark_as_read(
        &mut self,
        until_message_id: UiConversationMessageId,
        until_timestamp: DateTime<Utc>,
    ) {
        self.context
            .mark_as_read_tx
            .send_if_modified(|state| match state {
                MarkAsReadState::NotLoaded => {
                    warn!("Marking as read while conversation is not loaded");
                    false
                }
                MarkAsReadState::Marked { at }
                | MarkAsReadState::Scheduled {
                    until_timestamp: at,
                    ..
                } if *at < until_timestamp => {
                    *state = MarkAsReadState::Scheduled {
                        until_message_id,
                        until_timestamp,
                    };
                    true
                }
                MarkAsReadState::Marked { .. } => {
                    false // already marked as read
                }
                MarkAsReadState::Scheduled { .. } => {
                    false // already scheduled at a later timestamp
                }
            });
    }
}

/// Loads the intial state and listen to the changes
#[frb(ignore)]
#[derive(Clone)]
struct ConversationDetailsContext {
    store: CoreUser,
    state_tx: watch::Sender<ConversationDetailsState>,
    conversation_id: ConversationId,
    mark_as_read_tx: watch::Sender<MarkAsReadState>,
}

impl ConversationDetailsContext {
    fn new(
        store: CoreUser,
        state_tx: watch::Sender<ConversationDetailsState>,
        conversation_id: ConversationId,
    ) -> Self {
        let (mark_as_read_tx, _) = watch::channel(Default::default());
        Self {
            store,
            state_tx,
            conversation_id,
            mark_as_read_tx,
        }
    }

    fn spawn(
        self,
        store_notifications: impl Stream<Item = Arc<StoreNotification>> + Send + 'static,
        stop: CancellationToken,
    ) {
        spawn_from_sync(async move {
            self.load_and_emit_state().await;
            self.spawn_mark_as_read_debounce_loop();
            self.store_notifications_loop(store_notifications, stop)
                .await;
        });
    }

    async fn load_and_emit_state(&self) {
        let (details, last_read) = self.load_conversation_details().await.unzip();
        let members = if details.is_some() {
            self.members_of_conversation()
                .await
                .inspect_err(|error| error!(%error, "Failed fetching members"))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(last_read) = last_read {
            let _ = self.mark_as_read_tx.send_replace(MarkAsReadState::Marked {
                // truncate nanoseconds because they are not supported by Dart's DateTime
                at: last_read.trunc_subsecs(6),
            });
        }

        let new_state = ConversationDetailsState {
            conversation: details,
            members,
        };
        let _ = self.state_tx.send(new_state);
    }

    async fn load_conversation_details(&self) -> Option<(UiConversationDetails, DateTime<Utc>)> {
        let conversation = self.store.conversation(&self.conversation_id).await?;
        let last_read = conversation.last_read();
        Some((
            converation_into_ui_details(&self.store, conversation).await,
            last_read,
        ))
    }

    async fn members_of_conversation(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .store
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

    fn spawn_mark_as_read_debounce_loop(&self) {
        tokio::spawn(Self::mark_as_read_debounce_loop(
            self.mark_as_read_tx.subscribe(),
            self.store.clone(),
            self.conversation_id,
        ));
    }

    async fn mark_as_read_debounce_loop(
        mut mark_as_read_rx: watch::Receiver<MarkAsReadState>,
        store: CoreUser,
        conversation_id: ConversationId,
    ) {
        const MARK_AS_READ_DEBOUNCE_DURATION: Duration = Duration::from_secs(2);

        while let Ok(message_id) = debounced_watch(
            &mut mark_as_read_rx,
            MARK_AS_READ_DEBOUNCE_DURATION,
            MarkAsReadState::debounce_condition,
        )
        .await
        {
            info!(?message_id, "Marking as read (after debounce)");
            if let Err(error) = store
                .mark_conversation_as_read(conversation_id, message_id.into())
                .await
            {
                error!(%error, ?conversation_id, "Failed to mark as read");
            }
        }
    }
}

#[frb(ignore)]
#[derive(Debug, Clone, Default)]
enum MarkAsReadState {
    #[default]
    NotLoaded,
    /// Conversation is marked as read until the given timestamp
    Marked { at: DateTime<Utc> },
    /// Conversation is scheduled to be marked as read until the given timestamp and message id
    Scheduled {
        until_message_id: UiConversationMessageId,
        until_timestamp: DateTime<Utc>,
    },
}

impl MarkAsReadState {
    /// Condition when to emit a message id to mark it as read or continue debouncing.
    fn debounce_condition(state: MarkAsReadState) -> ControlFlow<UiConversationMessageId, Self> {
        match state {
            MarkAsReadState::NotLoaded | MarkAsReadState::Marked { at: _ } => {
                ControlFlow::Continue(state)
            }
            MarkAsReadState::Scheduled {
                until_message_id,
                until_timestamp: _,
            } => ControlFlow::Break(until_message_id),
        }
    }
}

/// Debounces the value in `rx` until the `condition` is met (`Break`) and `duration` has passed.
async fn debounced_watch<T: Default + Clone, R>(
    rx: &mut watch::Receiver<T>,
    duration: Duration,
    condition: impl Fn(T) -> ControlFlow<R, T>,
) -> Result<R, watch::error::RecvError> {
    let mut latest = T::default();
    loop {
        tokio::select! {
            res = rx.changed() => {
                res?; // short-circuit closed channel
                latest = rx.borrow_and_update().clone();
            }
            _ = tokio::time::sleep(duration) => {
                match condition(latest) {
                    ControlFlow::Continue(value) => {
                        latest = value;
                    }
                    ControlFlow::Break(value) => {
                        return Ok(value);
                    }
                }
            }
        }
    }
}
