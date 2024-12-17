// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use anyhow::bail;
use flutter_rust_bridge::frb;
use log::error;
use phnxcoreclient::clients::CoreUser;
use phnxcoreclient::ConversationId;
use phnxtypes::identifiers::SafeTryInto;
use tokio::sync::RwLock;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::util::{spawn_from_sync, SharedCubitSinks};
use crate::StreamSink;

use super::conversations::converation_into_ui_details;
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
        let shared = Shared::new(core_user.clone());
        let cancel = CancellationToken::new();

        // background task to load the conversation details and listen to changes
        spawn_from_sync({
            let shared = shared.clone();
            let cancel = cancel.clone();
            async move {
                if let Some(details) =
                    get_conversation_details_by_id(&shared.core_user, conversation_id).await
                {
                    let members = members_of_conversation(&shared.core_user, conversation_id)
                        .await
                        .inspect_err(|error| error!("Error when fetching members: {error}"))
                        .unwrap_or_default();
                    let new_state = ConversationDetailsState {
                        conversation: Some(details),
                        members,
                    };
                    shared
                        .emit(|state| {
                            *state = new_state;
                            Some(())
                        })
                        .await;
                }

                // TODO: Subscribe to changes from the store/server/websocket
                // <https://github.com/phnx-im/infra/issues/254>
                let _cancel = cancel;
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
        log::info!("ConversationDetailsCubitBase::state");
        // Note: don't lock too long, this is the UI thread
        self.shared.state.blocking_read().clone()
    }

    pub async fn stream(&mut self, sink: StreamSink<ConversationDetailsState>) {
        log::info!("ConversationDetailsCubitBase::stream");
        self.shared.sinks.push(sink).await;
    }

    // Cubit methods

    pub async fn set_conversation_picture(&mut self, bytes: Option<Vec<u8>>) -> anyhow::Result<()> {
        let conversation_id = self
            .shared
            .state
            .read()
            .await
            .conversation
            .as_ref()
            .map(|c| c.id);
        let Some(conversation_id) = conversation_id else {
            bail!("conversation not found");
        };
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

impl Drop for ConversationDetailsCubitBase {
    fn drop(&mut self) {
        log::info!("ConversationDetailsCubitBase::drop");
    }
}

#[frb(ignore)]
#[derive(Clone)]
struct Shared {
    state: Arc<RwLock<ConversationDetailsState>>,
    sinks: SharedCubitSinks<ConversationDetailsState>,
    core_user: CoreUser,
}

impl Shared {
    fn new(core_user: CoreUser) -> Self {
        Self {
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
