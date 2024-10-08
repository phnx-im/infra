// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;

use chrono::{DateTime, Utc};
use phnxcoreclient::{clients::CoreUser, ConversationId};

use super::mark_as_read_debouncer::MarkAsReadDebouncer;

/// Application state that's opaque to Dart, but that is used to keep various
/// pieces of state pertaining to the application logic.
///
/// Appstate contains only ephemeral data and does not need to be persisted.
pub(crate) struct AppState {
    mark_as_read_debouncers: MarkAsReadDebouncer,
    user: CoreUser,
}

impl AppState {
    /// Create a new `AppState` with no current conversation and no ongoing
    /// marking of messages as read.
    pub(crate) fn new(user: CoreUser) -> Self {
        Self {
            mark_as_read_debouncers: MarkAsReadDebouncer::new(),
            user,
        }
    }

    /// Mark the messages in the conversation with the given [`ConversationId`]
    /// older than the given [`TimeStamp`] as read.
    ///
    /// This mechanism is debounced to avoid marking messages as read too often.
    /// If there is no debouncing currently in progress, this function will
    /// start a new debouncing process and return only after it has finished.
    /// Otherwise it will return immediately.
    pub(crate) async fn mark_messages_read_debounced(
        &self,
        conversation_id: ConversationId,
        timestamp: DateTime<Utc>,
    ) {
        self.mark_as_read_debouncers
            .mark_as_read_debounced(self.user.clone(), conversation_id, timestamp)
            .await
    }

    /// If there is a debouncing process going on for the conversation with the
    /// given [`ConversationId`], immediately stop it and mark all messages as
    /// read.
    pub(crate) async fn flush_debouncer_state(&self) -> Result<()> {
        self.mark_as_read_debouncers
            .flush_debouncer_state(self.user.clone())
            .await
    }
}
