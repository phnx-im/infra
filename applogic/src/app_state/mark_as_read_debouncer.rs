// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcoreclient::{users::SelfUser, ConversationId};
use phnxtypes::time::TimeStamp;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::{anyhow, Result};

/// The default duration (in milliseconds) it takes for the process to mark all
/// messages as read.
const DEFAULT_DURATION: u64 = 2000;

/// The interval at which to check and decrement the duration of the marking of
/// messages as read state.
const DURATION_CHECK_INTERVAL: u64 = 500;

#[derive(Debug, Clone)]
struct ConversationDebouncerState {
    duration: u64,
    timestamp: TimeStamp,
}

impl ConversationDebouncerState {
    /// Create a new `ConversationDebouncerState` with the given `timestamp` and
    /// a default duration of 2 seconds.
    pub(super) fn new(timestamp: TimeStamp, duration: u64) -> Self {
        Self {
            duration,
            timestamp,
        }
    }

    /// Set the timestamp of the last message that's being marked as read.
    pub(super) fn set_timestamp(&mut self, timestamp: TimeStamp) {
        self.timestamp = timestamp;
    }

    /// Reset the duration of the marking as read.
    pub(super) fn set_duration(&mut self, duration: u64) {
        self.duration = duration;
    }

    /// Decrement the duration of the debouncer state by the default check interval.
    pub(super) fn decrement_duration(&mut self) {
        // We use `checked_sub` to avoid underflow and set the duration to zero
        // if underflow would occur.
        self.duration = self
            .duration
            .checked_sub(DURATION_CHECK_INTERVAL)
            .unwrap_or(0);
    }
}

/// A debouncer that marks messages as read in conversations.
pub(super) struct MarkAsReadDebouncer {
    // Indicator if there is currently a debouncing thread running. To avoid
    // deadlocks, locks on `debouncer_thread_running` may only be acquired after
    // locks on `conversation_debouncer_states`.
    debouncer_thread_running: Arc<Mutex<bool>>,
    // A map of conversation ids to the state of an ongoing debouncing process.
    conversation_debouncer_states: Arc<Mutex<HashMap<ConversationId, ConversationDebouncerState>>>,
    // The duration of the debouncing process.
    duration: u64,
}

impl MarkAsReadDebouncer {
    /// Create a new `MarkAsReadTimer` with the given `timestamp` and
    /// conversation id. The timer has a default duration of 2 seconds.
    pub(super) fn new() -> Self {
        Self {
            debouncer_thread_running: Arc::new(Mutex::new(false)),
            conversation_debouncer_states: Arc::new(Mutex::new(HashMap::new())),
            duration: DEFAULT_DURATION,
        }
    }

    #[cfg(test)]
    fn new_with_duration(duration: u64) -> Self {
        Self {
            debouncer_thread_running: Arc::new(Mutex::new(false)),
            conversation_debouncer_states: Arc::new(Mutex::new(HashMap::new())),
            duration,
        }
    }
}

impl MarkAsReadDebouncer {
    /// If there is a debouncer state for the given conversation id, immediately
    /// flush the state by marking all messages in the conversation older then
    /// the current timestamp as read and removing the state.
    ///
    /// If there is no debouncer state for the given conversation id, this
    /// function does nothing.
    pub(crate) fn flush_debouncer_state<T: MarkAsRead>(
        &self,
        user: T,
        conversation_id: ConversationId,
    ) -> Result<()> {
        let mut conversation_map = self
            .conversation_debouncer_states
            .lock()
            .map_err(|e| anyhow!("Mark as read debouncer mutex poisoned: {}", e))?;
        if let Some(debouncer_state) = conversation_map.remove(&conversation_id) {
            user.mark_as_read(conversation_id, debouncer_state.timestamp)?;
        };
        Ok(())
    }

    /// This function checks if there is already a timer running that operates
    /// the debouncer mechanism.
    ///
    /// If there isn't, it will create a new debouncer instance for the given
    /// [`ConversationId`] and start a timer
    ///
    /// If there is, it will either create a new debouncer instance for the
    /// given [`ConversationId`], or it will update the timestamp and reset the
    /// duration of the existing debouncer instance and return.
    ///
    /// A running timer will periodically check and decrement the duration of
    /// all debouncer instances. If a debouncer instance's duration reaches
    /// zero, the messages in the conversation will be marked as read up to the
    /// timestamp set in the debouncer instance.
    ///
    /// The timer will stop if the duration of all debouncer instances has
    /// reached zero.
    pub(crate) fn mark_as_read_debounced<T: MarkAsRead + Sync + Send + 'static>(
        &self,
        user: T,
        conversation_id: ConversationId,
        timestamp: TimeStamp,
    ) -> Result<()> {
        let mut conversation_debouncer_states = self
            .conversation_debouncer_states
            .lock()
            .map_err(|e| anyhow!("Mark as read debouncer mutex poisoned: {}", e))?;
        // We first check if there is already a thread running.
        let mut debouncer_thread_running = self
            .debouncer_thread_running
            .lock()
            .map_err(|e| anyhow!("Debouncer thread running mutex poisoned: {}", e))?;
        if *debouncer_thread_running {
            // If there is, we check if there already is an instance for the given conversation id.
            if let Some(debouncer) = conversation_debouncer_states.get_mut(&conversation_id) {
                // If there is, we update the timestamp and reset the duration.
                debouncer.set_timestamp(timestamp);
                debouncer.set_duration(self.duration);
            } else {
                // If there isn't, we create a new instance for the given conversation id.
                conversation_debouncer_states.insert(
                    conversation_id,
                    ConversationDebouncerState::new(timestamp, self.duration),
                );
            }
            // We now return since there already is a timer running.
            return Ok(());
        };

        // Since there is no thread running, the map must be empty, so we add an
        // entry for the conversation with the given id to the map.
        conversation_debouncer_states.insert(
            conversation_id,
            ConversationDebouncerState::new(timestamp, self.duration),
        );

        // We now spawn a thread that periodically gets a lock on the
        // conversation map to check and decrement the duration of all debouncer
        // states. If the duration of a conversation's debouncer state hits
        // zero, the thread will mark messages in that conversation older than
        // the debouncer state's current timestamp as read and remove that
        // debouncer state. If there are no more debouncer states, the thread
        // will terminate.
        // If an error occurs in the thread, we log it and return.
        *debouncer_thread_running = true;
        let debouncer_states_mutex = self.conversation_debouncer_states.clone();
        let debouncer_thread_running_mutex = self.debouncer_thread_running.clone();
        let duration = self.duration;

        thread::spawn(move || {
            debouncing_timer(
                debouncer_states_mutex,
                debouncer_thread_running_mutex,
                user,
                duration,
            )
        });
        Ok(())
    }
}

fn debouncing_timer<T: MarkAsRead + Sync + Send>(
    debouncer_states_mutex: Arc<Mutex<HashMap<ConversationId, ConversationDebouncerState>>>,
    debouncer_thread_running_mutex: Arc<Mutex<bool>>,
    user: T,
    duration: u64,
) {
    loop {
        // Wait for a bit.
        sleep(Duration::from_millis(duration));
        // Re-acquire the lock.
        let mut debouncer_states = match debouncer_states_mutex.lock() {
            Ok(states) => states,
            Err(e) => {
                log::error!("Mark as read debouncer mutex poisoned: {}", e);
                return;
            }
        };

        // Go through all the debouncer states and decrement their
        // duration.
        let keys: Vec<_> = debouncer_states.keys().copied().collect();
        for conversation_id in keys.into_iter() {
            // This must be Some.
            let Some(debouncer_state) = debouncer_states.get_mut(&conversation_id) else {
                log::error!("Can't find debouncer state");
                return;
            };
            debouncer_state.decrement_duration();
            // If the duration has reached zero, we remove the debouncer
            // state from the map and mark the messages in the conversation
            // as read.
            if debouncer_state.duration == 0 {
                let debouncer_state_timestamp = debouncer_state.timestamp;
                // We remove the conversation regardless of whether marking
                // the messages as read was successful or not. If it wasn't,
                // the function will have to be called again and the
                // messages may be marked as read then.
                debouncer_states.remove(&conversation_id);
                if let Err(e) = user.mark_as_read(conversation_id, debouncer_state_timestamp) {
                    log::error!("Failed to mark messages as read: {}", e);
                    return;
                };
            }
        }
        // Terminate if this was the last map entry.
        if debouncer_states.is_empty() {
            match debouncer_thread_running_mutex.lock() {
                Ok(mut debouncer_thread_running) => *debouncer_thread_running = false,

                Err(e) => {
                    log::error!("Debouncer thread running mutex poisoned: {}", e);
                }
            }
            return;
        }
    }
}

pub(crate) trait MarkAsRead {
    fn mark_as_read(&self, conversation_id: ConversationId, timestamp: TimeStamp) -> Result<()>;
}

impl MarkAsRead for Arc<Mutex<SelfUser>> {
    fn mark_as_read(&self, conversation_id: ConversationId, timestamp: TimeStamp) -> Result<()> {
        let user = self
            .lock()
            .map_err(|e| anyhow!("User mutex poisoned: {}", e))?;
        user.mark_as_read(conversation_id, timestamp)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        thread::sleep,
        time::Duration,
    };

    use anyhow::Result;
    use phnxcoreclient::ConversationId;
    use phnxtypes::time::TimeStamp;
    use uuid::Uuid;

    use super::MarkAsRead;

    struct TestUser {
        conversations: HashMap<ConversationId, TimeStamp>,
    }

    impl TestUser {
        fn new() -> Self {
            Self {
                conversations: HashMap::new(),
            }
        }

        fn new_conversation(&mut self, timestamp: TimeStamp) -> ConversationId {
            let conversation_id: ConversationId = Uuid::new_v4().into();
            self.conversations
                .insert(conversation_id.clone(), timestamp);
            conversation_id
        }

        fn get_conversation(&self, conversation_id: &ConversationId) -> Option<TimeStamp> {
            self.conversations.get(conversation_id).cloned()
        }
    }

    impl MarkAsRead for Arc<Mutex<TestUser>> {
        fn mark_as_read(
            &self,
            conversation_id: ConversationId,
            timestamp: TimeStamp,
        ) -> Result<()> {
            let mut user = self.lock().unwrap();
            let conversation = user.conversations.get_mut(&conversation_id).unwrap();
            *conversation = timestamp;
            Ok(())
        }
    }

    // Test the debouncer mechanism by issuing repeated calls to mark messages
    // as read in multiple conversations and checking if the messages are marked
    // as read after the debouncing process has finished.
    #[test]
    fn mark_as_read_debouncer() {
        // Let's make the duration sligtly shorter to speed up the test.
        let test_duration = 1000;
        let mut user = TestUser::new();
        let mark_as_read_debouncers = super::MarkAsReadDebouncer::new_with_duration(test_duration);
        // First a simple test with a single conversation. Does it change the timestamp?
        let old_timestamp = TimeStamp::now();
        let conversation_id = user.new_conversation(old_timestamp);
        sleep(std::time::Duration::from_millis(100));
        let new_timestamp = TimeStamp::now();
        assert!(new_timestamp.is_more_recent_than(&old_timestamp));

        let user_mutex = Arc::new(Mutex::new(user));
        mark_as_read_debouncers
            .mark_as_read_debounced(user_mutex.clone(), conversation_id, new_timestamp)
            .unwrap();

        // Wait for debouncer to finish
        sleep(std::time::Duration::from_millis(test_duration * 3));

        let mut user = user_mutex.lock().unwrap();
        assert_eq!(
            user.get_conversation(&conversation_id).unwrap(),
            new_timestamp
        );

        // Now let's test the debouncing mechanism by issuing multiple calls in
        // quick succession.
        let timestamp_1 = TimeStamp::now();
        let conversation_id = user.new_conversation(timestamp_1);
        sleep(std::time::Duration::from_millis(100));
        let timestamp_2 = TimeStamp::now();
        drop(user);
        // First call
        mark_as_read_debouncers
            .mark_as_read_debounced(user_mutex.clone(), conversation_id, timestamp_2)
            .unwrap();
        // Check that it wasn't set immediately
        let user = user_mutex.lock().unwrap();
        assert_eq!(
            user.get_conversation(&conversation_id).unwrap(),
            timestamp_1
        );
        drop(user);

        // Second call
        sleep(std::time::Duration::from_millis(100));
        let timestamp_3 = TimeStamp::now();
        mark_as_read_debouncers
            .mark_as_read_debounced(user_mutex.clone(), conversation_id, timestamp_3)
            .unwrap();
        // Check that it wasn't set immediately
        let user = user_mutex.lock().unwrap();
        assert_eq!(
            user.get_conversation(&conversation_id).unwrap(),
            timestamp_1
        );
        drop(user);
        // Wait for debouncer to finish
        sleep(Duration::from_millis(test_duration * 4));
        let user = user_mutex.lock().unwrap();
        assert_eq!(
            user.get_conversation(&conversation_id).unwrap(),
            timestamp_3
        );
    }
}
