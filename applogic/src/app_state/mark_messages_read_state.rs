// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcoreclient::ConversationId;
use phnxtypes::time::TimeStamp;

use crate::app_state::AppState;

use std::{ops::DerefMut, thread::sleep, time::Duration};

use anyhow::{anyhow, Result};

pub(super) struct MarkAsReadTimer {
    // The timestamp of the last message that's being marked as read.
    timestamp: TimeStamp,
    // The remaining duration (in milliseconds) of the marking as read operation.
    // Once this reaches zero, all messages with a timestamp less than or equal
    // to `timestamp` will be marked as read.
    duration: u64,
}

/// The default duration (in milliseconds) it takes for the process to mark all
/// messages as read.
const DEFAULT_DURATION: u64 = 2000;

/// The interval at which to check and decrement the duration of the marking of
/// messages as read state.
const DURATION_CHECK_INTERVAL: u64 = 500;

impl MarkAsReadTimer {
    /// Create a new `MarkAsReadTimer` with the given `timestamp` and
    /// conversation id. The timer has a default duration of 2 seconds.
    pub(super) fn new(timestamp: TimeStamp) -> Self {
        // Duration in milliseconds.
        let duration = DEFAULT_DURATION;
        Self {
            timestamp,
            duration,
        }
    }

    pub(super) fn reset_duration(&mut self) {
        self.duration = DEFAULT_DURATION;
    }

    pub(super) fn set_timestamp(&mut self, timestamp: TimeStamp) {
        self.timestamp = timestamp;
    }

    pub(super) fn timestamp(&self) -> TimeStamp {
        self.timestamp
    }
}

pub(crate) enum MarkAsReadTimerState {
    /// The timer is currently running.
    Running,
    /// The timer with the conversation id has been stopped and messages older
    /// than the timestamp can be marked as read.
    Stopped(ConversationId, TimeStamp),
    /// The current conversation has changed and the timer has been cancelled.
    Cancelled,
}

impl AppState {
    /// Start a timer that indicates when messages in the current conversation
    /// should be marked as read.
    ///
    /// This function returns a `MarkAsReadTimerState` that indicates the state
    /// of the timer
    /// - `Running` if the timer is still running and has been updated with the
    ///   new timestamp.
    /// - `Stopped` if the timer has reached zero and has been stopped. At this
    ///   point, all messages with a timestamp less than or equal to the
    ///   timestamp that was set can be marked as read.
    ///
    /// This function initially gets a lock on the current conversation and then
    /// periodically gets a lock on the timer state, though not at the same
    /// time.
    ///
    /// Returns an error if there is no current conversation.
    pub(crate) fn set_mark_messages_as_read_timer(
        &self,
        timestamp: TimeStamp,
    ) -> Result<MarkAsReadTimerState> {
        // Get the current conversation state.
        let mut current_conversation_option = self
            .current_conversation
            .lock()
            .map_err(|e| anyhow!("Current conversation mutex poisoned: {}", e))?;

        // If there is no current conversation, there is nothing to do.
        let Some(current_conversation_state) = current_conversation_option.deref_mut() else {
            return Err(anyhow!("No current conversation"));
        };

        // Store the conversation id for later.
        let conversation_id = current_conversation_state.conversation_id();

        // We first check if there is already a timer running.
        let mark_as_read_state_option = current_conversation_state.mark_as_read_state_mut();
        if let Some(mark_as_read_state) = mark_as_read_state_option {
            // If there is already a timer running, we update the timestamp and
            // reset the duration (if the new timestamp is newer than the
            // current one).
            if timestamp.is_more_recent_than(&mark_as_read_state.timestamp) {
                mark_as_read_state.set_timestamp(timestamp);
                mark_as_read_state.reset_duration();
            }
            return Ok(MarkAsReadTimerState::Running);
        }

        // If there is no timer running, we start a new one.
        let _ = mark_as_read_state_option.insert(MarkAsReadTimer::new(timestamp));
        // Drop the lock to the current conversation s.t. other processes can
        // access the state.
        drop(current_conversation_option);

        // We now enter a loop where we periodically get a lock on the timer
        // state to check and decrement the duration. We do that until one of
        // two things happen: Either the duration has reached zero, or the
        // current conversation id has changed, upon which we immediately return
        // to report that the timer has stopped.
        loop {
            let waiting_interval = DURATION_CHECK_INTERVAL;
            // Wait for a bit.
            sleep(Duration::from_millis(waiting_interval));
            // Re-acquire the lock.
            let mut current_conversation_option = self
                .current_conversation
                .lock()
                .map_err(|e| anyhow!("Mark as read timer mutex poisoned: {}", e))?;
            // Let's first check the current conversation.
            // If there is no current conversation, we cancel the timer.
            let Some(current_conversation_state) = current_conversation_option.deref_mut() else {
                return Ok(MarkAsReadTimerState::Cancelled);
            };
            // There really should be a timer state, but if there isn't for some
            // reason, we just cancel the timer.
            let Some(mark_as_read_state) = current_conversation_state.mark_as_read_state_mut()
            else {
                return Ok(MarkAsReadTimerState::Cancelled);
            };

            // Decrement the duration by the amount of time we've waited.
            mark_as_read_state.duration -= waiting_interval;

            // Check if there is any time left.
            if mark_as_read_state.duration <= 0 {
                let current_timestamp = mark_as_read_state.timestamp;
                // Time's up. Let's delete the timer state and report that the
                // timer has stopped.
                current_conversation_state.delete_current_timer();
                return Ok(MarkAsReadTimerState::Stopped(
                    conversation_id,
                    current_timestamp,
                ));
            }
            // Otherwise keep looping.
        }
    }
}
