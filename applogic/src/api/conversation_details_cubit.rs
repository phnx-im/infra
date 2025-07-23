// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A single conversation details feature

use std::path::PathBuf;
use std::{sync::Arc, time::Duration};

use chrono::{DateTime, SubsecRound, Utc};
use flutter_rust_bridge::frb;
use mimi_content::{MessageStatus, MimiContent};
use mimi_room_policy::{MimiProposal, RoleIndex, VerifiedRoomState};
use phnxcommon::{OpenMlsRand, RustCrypto, identifiers::UserId};
use phnxcoreclient::{ConversationId, store::StoreNotification};
use phnxcoreclient::{ConversationMessageId, clients::CoreUser, store::Store};
use tls_codec::Serialize;
use tokio::{sync::watch, time::sleep};
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::api::types::UiMessageDraft;
use crate::message_content::MimiContentExt;
use crate::util::{Cubit, CubitCore, spawn_from_sync};
use crate::{StreamSink, api::types::UiMessageDraftSource};

use super::{
    conversation_list_cubit::load_conversation_details,
    types::{UiConversationDetails, UiUserId},
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
    pub members: Vec<UiUserId>,
    pub room_state: Option<UiRoomState>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UiRoomState {
    our_user: UserId,
    state: VerifiedRoomState,
}

impl UiRoomState {
    #[frb(sync)]
    pub fn can_kick(&self, target: &UiUserId) -> bool {
        let Ok(user) = self.our_user.tls_serialize_detached() else {
            return false;
        };
        let Ok(target) = UserId::from(target.clone()).tls_serialize_detached() else {
            return false;
        };

        self.state
            .can_apply_regular_proposals(
                &user,
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
        let store = user_cubit.core_user().clone();
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
        let salt: [u8; 16] = RustCrypto::default().random_array()?;
        let content = MimiContent::simple_markdown_message(message_text, salt);

        let mut draft = None;
        self.core.state_tx().send_if_modified(|state| {
            let Some(conversation) = state.conversation.as_mut() else {
                return false;
            };
            draft = conversation.draft.take();
            draft.is_some()
        });

        // Remove stored draft
        if draft.is_some() {
            self.context
                .store
                .store_message_draft(self.context.conversation_id, None)
                .await?;
        }
        let editing_id = draft.and_then(|d| d.editing_id);

        self.context
            .store
            .send_message(self.context.conversation_id, content, editing_id)
            .await
            .inspect_err(|error| error!(%error, "Failed to send message"))?;

        Ok(())
    }

    pub async fn upload_attachment(&self, path: String) -> anyhow::Result<()> {
        let path = PathBuf::from(path);
        self.context
            .store
            .upload_attachment(self.context.conversation_id, &path)
            .await?;
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
            .send_if_modified(|state| match &state {
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

        let (_, read_mimi_ids) = self
            .context
            .store
            .mark_conversation_as_read(self.context.conversation_id, until_message_id)
            .await?;

        let statuses = read_mimi_ids
            .iter()
            .map(|mimi_id| (mimi_id, MessageStatus::Read));
        if let Err(error) = self
            .context
            .store
            .send_delivery_receipts(self.context.conversation_id, statuses)
            .await
        {
            error!(%error, "Failed to send delivery receipt");
        }

        Ok(())
    }

    pub async fn store_draft(&self, draft_message: String) -> anyhow::Result<()> {
        let changed = self.core.state_tx().send_if_modified(|state| {
            let Some(conversation) = state.conversation.as_mut() else {
                return false;
            };
            match &mut conversation.draft {
                Some(draft) if draft.message != draft_message => {
                    draft.message = draft_message;
                    draft.updated_at = Utc::now();
                    true
                }
                Some(_) => false,
                None => {
                    conversation.draft.replace(UiMessageDraft::new(
                        draft_message,
                        UiMessageDraftSource::User,
                    ));
                    true
                }
            }
        });
        if changed {
            self.store_draft_from_state().await?;
        }
        Ok(())
    }

    pub async fn reset_draft(&self) {
        self.core.state_tx().send_if_modified(|state| {
            let Some(conversation) = state.conversation.as_mut() else {
                return false;
            };
            conversation.draft.take().is_some()
        });
    }

    pub async fn edit_message(
        &self,
        message_id: Option<ConversationMessageId>,
    ) -> anyhow::Result<()> {
        // Load message
        let message = match message_id {
            Some(message_id) => self.context.store.message(message_id).await?,
            None => {
                self.context
                    .store
                    .last_message_by_user(
                        self.context.conversation_id,
                        self.context.store.user_id(),
                    )
                    .await?
            }
        };
        let Some(message) = message else {
            return Ok(());
        };

        // Get plain body if any; if none, this message is not editable.
        let Some(body) = message
            .message()
            .mimi_content()
            .and_then(|content| content.plain_body())
        else {
            return Ok(());
        };

        // Update draft in state
        let changed = self.core.state_tx().send_if_modified(|state| {
            let Some(conversation) = state.conversation.as_mut() else {
                return false;
            };
            let draft = conversation.draft.get_or_insert_with(|| UiMessageDraft {
                message: String::new(),
                editing_id: None,
                updated_at: Utc::now(),
                source: UiMessageDraftSource::System,
            });
            if draft.editing_id.is_some() {
                return false;
            }
            draft.message = body.to_owned();
            draft.editing_id.replace(message.id());
            true
        });

        if changed {
            self.store_draft_from_state().await?;
        }

        Ok(())
    }

    async fn store_draft_from_state(&self) -> anyhow::Result<()> {
        let draft = self
            .core
            .state_tx()
            .borrow()
            .conversation
            .as_ref()
            .and_then(|c| c.draft.clone());
        self.context
            .store
            .store_message_draft(
                self.context.conversation_id,
                draft.map(|d| d.into_draft()).as_ref(),
            )
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
        store_notifications: impl Stream<Item = Arc<StoreNotification>> + Send + Unpin + 'static,
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
        let details = load_conversation_details(&self.store, conversation).await;
        Some((details, last_read))
    }

    async fn members_of_conversation(&self) -> anyhow::Result<Vec<UiUserId>> {
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
        mut store_notifications: impl Stream<Item = Arc<StoreNotification>> + Unpin,
        stop: CancellationToken,
    ) {
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
