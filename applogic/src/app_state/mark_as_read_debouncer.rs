// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use tokio::{sync::Mutex, time::sleep};

use phnxcoreclient::{clients::CoreUser, ConversationId};

use anyhow::{anyhow, Result};
use tracing::error;

/// The default duration (in milliseconds) it takes for the process to mark all
/// messages as read.
const DEFAULT_DURATION: u64 = 2000;

/// The interval at which to check and decrement the duration of the marking of
/// messages as read state.
const DURATION_CHECK_INTERVAL: u64 = 500;

#[derive(Debug)]
struct DebouncerState {
    // A map of conversation ids to the state of an ongoing debouncing process.
    // If this is `None`, then there is no debouncing thread running.
    conversation_timestamps: HashMap<ConversationId, DateTime<Utc>>,
    // The duration of the debouncing process.
    duration: u64,
    starting_duration: u64,
}

impl DebouncerState {
    /// Reset the duration of the debouncer.
    pub(super) fn reset_duration(&mut self) {
        self.duration = self.starting_duration;
    }

    /// Decrement the duration of the debouncer state by the default check interval.
    pub(super) fn decrement_duration(&mut self) {
        // We use `checked_sub` to avoid underflow and set the duration to zero
        // if underflow would occur.
        self.duration = self.duration.saturating_sub(DURATION_CHECK_INTERVAL);
    }

    /// Create a new [`DebouncerState`] with the given `timestamp` and
    /// conversation id, as well as the default duration.
    fn new(
        conversation_timestamps: impl Into<HashMap<ConversationId, DateTime<Utc>>>,
        default_duration: u64,
    ) -> Self {
        Self {
            conversation_timestamps: conversation_timestamps.into(),
            duration: default_duration,
            starting_duration: default_duration,
        }
    }
}

/// A debouncer that marks messages as read in conversations.
pub(super) struct MarkAsReadDebouncer {
    conversation_debouncer_states_option: Arc<Mutex<Option<DebouncerState>>>,
    default_duration: u64,
}

impl MarkAsReadDebouncer {
    /// Create a new [`MarkAsReadDebouncer`] with the given `timestamp` and
    /// conversation id. The timer has a default duration of 2 seconds.
    pub(super) fn new() -> Self {
        Self {
            conversation_debouncer_states_option: Arc::new(Mutex::new(None)),
            default_duration: DEFAULT_DURATION,
        }
    }

    /*
     Commented out for now, until https://github.com/rust-lang/rust/issues/100013 is resolved.
    #[cfg(test)]
     fn new_with_duration(duration: u64) -> Self {
         Self {
             conversation_debouncer_states_option: Arc::new(Mutex::new(None)),
             default_duration: duration,
         }
     } */

    /// If there is a debouncer state for the given conversation id, immediately
    /// flush the state by marking all messages in the conversation older then
    /// the current timestamp as read and removing the state.
    ///
    /// If there is no debouncer state for the given conversation id, this
    /// function does nothing.
    pub(crate) async fn flush_debouncer_state<T: MarkAsRead>(&self, user: T) -> Result<()> {
        let mut debouncer_state_option = self.conversation_debouncer_states_option.lock().await;
        if let Some(debouncer_state) = debouncer_state_option.take() {
            user.mark_as_read(debouncer_state.conversation_timestamps)
                .await?;
            debouncer_state_option.take();
        }
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
    pub(crate) async fn mark_as_read_debounced(
        &self,
        user: CoreUser, // impl MarkAsRead + Sync + Send + 'static
        conversation_id: ConversationId,
        timestamp: DateTime<Utc>,
    ) {
        let mut conversation_debouncer_state_option =
            self.conversation_debouncer_states_option.lock().await;
        // Check if there is already a debouncer state.
        if let Some(ref mut debouncer_state) = *conversation_debouncer_state_option {
            // As there is a debouncer state, there must already be a thread
            // running, so all we have to do is update (or add) the timestamp
            // and reset the duration.
            // We only insert the timestamp if it is more recent than the
            // existing timestamp.
            if let Some(existing_timestamp) = debouncer_state
                .conversation_timestamps
                .get(&conversation_id)
            {
                if timestamp <= *existing_timestamp {
                    return;
                }
            }

            debouncer_state
                .conversation_timestamps
                .insert(conversation_id, timestamp);
            debouncer_state.reset_duration();
            return;
        }

        // Since there is no thread running we create a new state and start a new thread.
        let debouncer_state =
            DebouncerState::new([(conversation_id, timestamp)], self.default_duration);
        *conversation_debouncer_state_option = Some(debouncer_state);

        // We now spawn a thread that periodically gets a lock on the debouncer
        // state to check and decrement the duration of all conversation. If the
        // duration of a conversation's debouncer state hits zero, the thread
        // will mark messages in that conversation older than the conversation
        // debouncer state's current timestamp as read and remove that
        // conversation debouncer state. If there are no more conversation
        // debouncer states, the thread will terminate. If an error occurs in
        // the thread, we log it and return.
        let debouncer_state_mutex = self.conversation_debouncer_states_option.clone();

        tokio::spawn(async move { debouncing_timer(debouncer_state_mutex.clone(), user).await });
    }
}

async fn debouncing_timer(
    debouncer_state_mutex: Arc<Mutex<Option<DebouncerState>>>,
    user: CoreUser, // impl MarkAsRead + Sync + Send + 'static
) {
    loop {
        // Wait for a bit.
        sleep(Duration::from_millis(DURATION_CHECK_INTERVAL)).await;
        // Re-acquire the lock.
        let mut debouncer_state_option = debouncer_state_mutex.lock().await;

        // If the debouncer state was removed while the debouncer thread was
        // running (e.g. because the debouncer state was flushed), there is
        // nothing left for the thread to do.
        let Some(ref mut debouncer_state) = *debouncer_state_option else {
            return;
        };

        debouncer_state.decrement_duration();
        // If the duration has reached zero, we mark the messages as read
        // and remove the debouncer state.
        if debouncer_state.duration == 0 {
            if let Err(error) = user
                .mark_as_read(debouncer_state.conversation_timestamps.clone())
                .await
            {
                error!(%error, "Failed to mark messages as read");
            };
            debouncer_state_option.take();
            return;
        }
    }
}

pub(crate) trait MarkAsRead {
    async fn mark_as_read<T: IntoIterator<Item = (ConversationId, DateTime<Utc>)> + Send>(
        &self,
        mark_as_read_data: T,
    ) -> Result<()>;
}

impl MarkAsRead for CoreUser {
    async fn mark_as_read<T: IntoIterator<Item = (ConversationId, DateTime<Utc>)> + Send>(
        &self,
        mark_as_read_data: T,
    ) -> Result<()> {
        self.mark_as_read(mark_as_read_data)
            .await
            .map_err(|e| anyhow!("Error: {:?}", e))
            .unwrap();
        Ok(())
    }
}
/*
Commented out for now, until https://github.com/rust-lang/rust/issues/100013 is resolved.
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use anyhow::Result;
    use phnxcoreclient::ConversationId;
    use phnxtypes::time::TimeStamp;
    use tokio::sync::Mutex;
    use tokio::time::sleep;
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
        async fn mark_as_read<
            'b,
            T: 'b + IntoIterator<Item = (&'b ConversationId, &'b TimeStamp)>,
        >(
            &self,
            mark_as_read_data: T,
        ) -> Result<()> {
            let mut user = self.lock().await
            for (conversation_id, timestamp) in mark_as_read_data {
                let conversation = user.conversations.get_mut(&conversation_id).unwrap();
                *conversation = *timestamp;
            }
            Ok(())
        }
    }

    // Test the debouncer mechanism by issuing repeated calls to mark messages
    // as read in multiple conversations and checking if the messages are marked
    // as read after the debouncing process has finished.
    #[tokio::test]
    async fn mark_as_read_debouncer() {
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
            .await;

        // Wait for debouncer to finish
        sleep(std::time::Duration::from_millis(test_duration * 3));

        let mut user = user_mutex.lock().await
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
            .await;
        // Check that it wasn't set immediately
        let user = user_mutex.lock().await
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
            .await;
        // Check that it wasn't set immediately
        let user = user_mutex.lock().await
        assert_eq!(
            user.get_conversation(&conversation_id).unwrap(),
            timestamp_1
        );
        drop(user);
        // Wait for debouncer to finish
        sleep(Duration::from_millis(test_duration * 4));
        let user = user_mutex.lock().await
        assert_eq!(
            user.get_conversation(&conversation_id).unwrap(),
            timestamp_3
        );
    }
}
 */
