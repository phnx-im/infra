// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use log::error;
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::util::spawn_from_sync;
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

        spawn_from_sync(load_conversation_and_listen(
            core_user.clone(),
            state_tx.clone(),
            cancel.clone(),
            conversation_id,
        ));

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

async fn load_conversation_and_listen(
    core_user: CoreUser,
    state_tx: watch::Sender<ConversationDetailsState>,
    stop: CancellationToken,
    conversation_id: ConversationId,
) {
    if let Some(details) = get_conversation_details_by_id(&core_user, conversation_id).await {
        let members = members_of_conversation(&core_user, conversation_id)
            .await
            .inspect_err(|error| error!("Error when fetching members: {error}"))
            .unwrap_or_default();
        let new_state = ConversationDetailsState {
            conversation: Some(details),
            members,
        };
        if state_tx.send(new_state).is_err() {
            return;
        }
    }

    // TODO: Subscribe to changes from the store/server/websocket
    // <https://github.com/phnx-im/infra/issues/254>
    let _stop = stop;
}

async fn get_conversation_details_by_id(
    core_user: &CoreUser,
    conversation_id: ConversationId,
) -> Option<UiConversationDetails> {
    let conversation = core_user.conversation(&conversation_id).await?;
    Some(converation_into_ui_details(core_user, conversation).await)
}

async fn members_of_conversation(
    core_user: &CoreUser,
    conversation_id: ConversationId,
) -> anyhow::Result<Vec<String>> {
    Ok(core_user
        .conversation_participants(conversation_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.to_string())
        .collect())
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
