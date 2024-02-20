// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use anyhow::Result;

use phnxcoreclient::{users::SelfUser, ConversationId};
use phnxtypes::time::TimeStamp;

use self::mark_as_read_debouncer::MarkAsReadDebouncer;

pub(crate) mod mark_as_read_debouncer;

/// Application state that's opaque to Dart, but that is used to keep various
/// pieces of state pertaining to the application logic.
///
/// Appstate contains only ephemeral data and does not need to be persisted.
pub(super) struct AppState {
    mark_as_read_debouncers: MarkAsReadDebouncer,
}

impl AppState {
    /// Create a new `AppState` with no current conversation and no ongoing
    /// marking of messages as read.
    pub(super) fn new() -> Self {
        Self {
            mark_as_read_debouncers: MarkAsReadDebouncer::new(),
        }
    }

    /// Mark the messages in the conversation with the given [`ConversationId`]
    /// older than the given [`TimeStamp`] as read.
    ///
    /// This mechanism is debounced to avoid marking messages as read too often.
    /// If there is no debouncing currently in progress, this function will
    /// start a new debouncing process and return only after it has finished.
    /// Otherwise it will return immediately.
    pub(super) fn mark_messages_read_debounced(
        &self,
        user_mutex: Arc<Mutex<SelfUser>>,
        conversation_id: ConversationId,
        timestamp: TimeStamp,
    ) -> Result<()> {
        self.mark_as_read_debouncers
            .mark_as_read_debounced(user_mutex, conversation_id, timestamp)
    }

    /// If there is a debouncing process going on for the conversation with the
    /// given [`ConversationId`], immediately stop it and mark all messages as
    /// read.
    pub(super) fn flush_debouncer_state(&self, user_mutex: Arc<Mutex<SelfUser>>) -> Result<()> {
        self.mark_as_read_debouncers
            .flush_debouncer_state(user_mutex)
    }
}
