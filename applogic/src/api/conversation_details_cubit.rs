// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A single conversation details feature

use mimi_room_policy::{MimiProposal, RoleIndex, VerifiedRoomState};

use std::{sync::Arc, time::Duration};

use chrono::{DateTime, SubsecRound, Utc};
use flutter_rust_bridge::frb;
use mimi_content::MimiContent;
use phnxcoreclient::{ConversationId, store::StoreNotification};
use phnxcoreclient::{ConversationMessageId, clients::CoreUser, store::Store};
use tokio::{sync::watch, time::sleep};
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::StreamSink;
use crate::util::{Cubit, CubitCore, spawn_from_sync};

use super::{
    conversation_list_cubit::converation_into_ui_details,
    types::{UiClientId, UiConversationDetails},
    user_cubit::UserCubitBase,
};

/// The state of a single conversation
///
/// Contains the conversation details and the list of members.
///
/// Also see [`ConversationDetailsCubitBase`].
#[frb(dart_metadata = ("freezed"))]
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ConversationDetailsState {
    pub conversation: Option<UiConversationDetails>,
    pub members: Vec<UiClientId>,
    pub room_state: Option<UiRoomState>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiRoomState {
    our_user: u32,
    state: VerifiedRoomState,
}

impl UiRoomState {
    #[frb(sync)]
    pub fn can_kick(&self, target: u32) -> bool {
        self.state
            .can_apply_regular_proposals(
                &self.our_user,
                &[MimiProposal::ChangeRole {
                    target,
                    role: RoleIndex::Outsider,
                }],
            )
            .is_ok()
    }
}

/// The cubit responsible for a single conversation
///
/// Fetches the conversation details and the list of members. Allows to modify the conversation
/// details, send messages and mark the conversation as read up to a given message.
#[frb(opaque)]
pub struct ConversationDetailsCubitBase {
    context: ConversationDetailsContext,
    core: CubitCore<ConversationDetailsState>,
}

impl ConversationDetailsCubitBase {
    /// Creates a new cubit for the given conversation.
    ///
    /// The cubit will fetch the conversation details and the list of members. It will also listen
    /// to the changes in the conversation and update the state accordingly.
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

    /// Sets the conversation picture.
    ///
    /// When `bytes` is `None`, the conversation picture is removed.
    pub async fn set_conversation_picture(&mut self, bytes: Option<Vec<u8>>) -> anyhow::Result<()> {
        Store::set_conversation_picture(
            &self.context.store,
            self.context.conversation_id,
            bytes.clone(),
        )
        .await
    }

    /// Sends a message to the conversation.
    ///
    /// The not yet sent message is immediately stored in the local store and then the message is
    /// send to the DS.
    pub async fn send_message(&self, message_text: String) -> anyhow::Result<()> {
        let content = MimiContent::simple_markdown_message(message_text);

        self.context
            .store
            .send_message(self.context.conversation_id, content)
            .await
            .inspect_err(|error| error!(%error, "Failed to send message"))?;

        Ok(())
    }

    /// Marks the conversation as read until the given message id (including).
    ///
    /// The calls to this method are debounced with a fixed delay.
    pub async fn mark_as_read(
        &self,
        until_message_id: ConversationMessageId,
        until_timestamp: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let scheduled = self
            .context
            .mark_as_read_tx
            .send_if_modified(|state| match state {
                MarkAsReadState::NotLoaded => {
                    error!("Marking as read while conversation is not loaded");
                    false
                }
                MarkAsReadState::Marked { at }
                | MarkAsReadState::Scheduled {
                    until_timestamp: at,
                    until_message_id: _,
                } if *at < until_timestamp => {
                    *state = MarkAsReadState::Scheduled {
                        until_timestamp,
                        until_message_id,
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
        if !scheduled {
            return Ok(());
        }

        // debounce
        const MARK_AS_READ_DEBOUNCE: Duration = Duration::from_secs(2);
        let mut rx = self.context.mark_as_read_tx.subscribe();
        tokio::select! {
            _ = rx.changed() => return Ok(()),
            _ = sleep(MARK_AS_READ_DEBOUNCE) => {},
        };

        // check if the scheduled state is still valid and if so, mark it as read
        let scheduled = self
            .context
            .mark_as_read_tx
            .send_if_modified(|state| match state {
                MarkAsReadState::Scheduled {
                    until_message_id: scheduled_message_id,
                    until_timestamp,
                } if *scheduled_message_id == until_message_id => {
                    *state = MarkAsReadState::Marked {
                        at: *until_timestamp,
                    };
                    true
                }
                _ => false,
            });
        if !scheduled {
            return Ok(());
        }

        self.context
            .store
            .mark_conversation_as_read(self.context.conversation_id, until_message_id)
            .await?;
        Ok(())
    }
}

/// Loads the initial state and listen to the changes
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
        let room_state = if let Some(details) = &details {
            if let Ok((our_id, state)) = self.store.load_room_state(&details.id).await {
                Some(UiRoomState {
                    our_user: our_id,
                    state,
                })
            } else {
                None
            }
        } else {
            None
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
            room_state,
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

    async fn members_of_conversation(&self) -> anyhow::Result<Vec<UiClientId>> {
        Ok(self
            .store
            .conversation_participants(self.conversation_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(From::from)
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

#[frb(ignore)]
#[derive(Debug, Default)]
enum MarkAsReadState {
    #[default]
    NotLoaded,
    /// Conversation is marked as read until the given timestamp
    Marked { at: DateTime<Utc> },
    /// Conversation is scheduled to be marked as read until the given timestamp and message id
    Scheduled {
        until_timestamp: DateTime<Utc>,
        until_message_id: ConversationMessageId,
    },
}
