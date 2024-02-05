// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcoreclient::ConversationId;
use phnxtypes::time::TimeStamp;

use super::mark_messages_read_state::MarkAsReadTimer;

pub(crate) struct CurrentConversationState {
    // The conversation that's currently being viewed.
    conversation_id: ConversationId,
    // The state of an ongoing marking of messages as read.
    mark_as_read_state: Option<MarkAsReadTimer>,
}

impl CurrentConversationState {
    /// Create a new `CurrentConversationState` with the given `conversation_id`.
    pub(crate) fn new(conversation_id: ConversationId) -> Self {
        Self {
            conversation_id,
            mark_as_read_state: None,
        }
    }

    /// Get a mutable reference to the current mark as read state option.
    pub(super) fn mark_as_read_state_mut(&mut self) -> &mut Option<MarkAsReadTimer> {
        &mut self.mark_as_read_state
    }

    /// This deletes the current timer (if any) and returns the timestamp of the
    /// last message to be marked as read.
    pub(crate) fn delete_current_timer(&mut self) -> Option<TimeStamp> {
        let timestamp = self
            .mark_as_read_state
            .as_ref()
            .map(|timer| timer.timestamp());
        self.mark_as_read_state = None;
        timestamp
    }

    /// Return the current conversation id
    pub(crate) fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }
}
