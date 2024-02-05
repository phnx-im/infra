// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;

use self::current_conversation_state::CurrentConversationState;

pub(crate) mod current_conversation_state;
pub(crate) mod mark_messages_read_state;

/// Application state that's opaque to Dart, but that is used to keep various
/// pieces of state pertaining to the application logic.
///
/// Appstate contains only ephemeral data and does not need to be persisted.
pub(super) struct AppState {
    // The conversation that's currently being viewed and associated state.
    pub(super) current_conversation: Mutex<Option<CurrentConversationState>>,
}

impl AppState {
    /// Create a new `AppState` with no current conversation and no ongoing
    /// marking of messages as read.
    pub(super) fn new() -> Self {
        Self {
            current_conversation: Mutex::new(None),
        }
    }
}
