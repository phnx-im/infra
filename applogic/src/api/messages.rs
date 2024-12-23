// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    clients::process::process_qs::ProcessedQsMessages, ConversationId, ConversationMessage,
    Message, MimiContent,
};
use tokio::sync::broadcast;

use crate::notifier::{dispatch_conversation_notifications, dispatch_message_notifications};

use super::{
    notifications::LocalNotificationContent,
    types::{UiConversationMessage, UiMessage},
    user::User,
};

#[frb(ignore)]
#[derive(Debug, Default)]
pub(crate) struct FetchedMessages {
    pub(crate) new_conversations: Vec<ConversationId>,
    pub(crate) changed_conversations: Vec<ConversationId>,
    pub(crate) new_messages: Vec<ConversationMessage>,
    pub(crate) notifications_content: Vec<LocalNotificationContent>,
}

impl User {
    /// Fetch AS messages
    pub(crate) async fn fetch_as_messages(&self) -> Result<Vec<ConversationId>> {
        let as_messages = self.user.as_fetch_messages().await?;

        // Process each as message individually and dispatch conversation
        // notifications to the UI in case a new conversation is created.
        let mut new_connections = vec![];
        for as_message in as_messages {
            let as_message_plaintext = self.user.decrypt_as_queue_message(as_message).await?;
            let conversation_id = self.user.process_as_message(as_message_plaintext).await?;
            new_connections.push(conversation_id);
        }

        Ok(new_connections)
    }

    /// Fetch QS messages
    pub(crate) async fn fetch_qs_messages(&self) -> Result<ProcessedQsMessages> {
        let qs_messages = self.user.qs_fetch_messages().await?;
        self.user.fully_process_qs_messages(qs_messages).await
    }

    /// Fetch both AS and QS messages
    pub(crate) async fn fetch_all_messages(&self) -> Result<FetchedMessages> {
        let mut notifications = Vec::new();

        // Fetch AS connection requests
        let new_connections = self.fetch_as_messages().await?;
        notifications.extend(
            self.new_connection_request_notifications(&new_connections)
                .await,
        );

        // Fetch QS messages
        let ProcessedQsMessages {
            new_conversations,
            changed_conversations,
            new_messages,
        } = self.fetch_qs_messages().await?;

        notifications.extend(
            self.new_conversation_notifications(&new_conversations)
                .await,
        );
        notifications.extend(self.new_message_notifications(&new_messages).await);

        Ok(FetchedMessages {
            new_conversations,
            changed_conversations,
            new_messages,
            notifications_content: notifications,
        })
    }

    /// Fetch all messages and dispatch them to the UI and desktop
    pub async fn fetch_messages(&self) -> Result<()> {
        let fetched_messages = self.fetch_all_messages().await?;

        // Send a notification to the OS (desktop only)
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        crate::notifier::show_desktop_notifications(&fetched_messages.notifications_content);

        // Let the UI know there is new stuff
        tokio::join!(
            dispatch_message_notifications(&self.notification_hub, fetched_messages.new_messages),
            dispatch_conversation_notifications(
                &self.notification_hub,
                fetched_messages.new_conversations
            ),
            dispatch_conversation_notifications(
                &self.notification_hub,
                fetched_messages.changed_conversations
            ),
        );

        Ok(())
    }

    pub async fn send_message(
        &self,
        conversation_id: ConversationId,
        message: String,
    ) -> Result<UiConversationMessage> {
        let content = MimiContent::simple_markdown_message(self.user.user_name().domain(), message);
        self.user
            .send_message(conversation_id, content)
            .await
            .map(|m| m.into())
    }

    pub async fn get_messages(
        &self,
        conversation_id: ConversationId,
        last_n: u32,
    ) -> Vec<UiConversationMessage> {
        let messages = self
            .user
            .get_messages(conversation_id, last_n as usize)
            .await
            .unwrap_or_default();

        group_messages(messages)
    }

    /// This function is called from the flutter side to mark messages as read.
    ///
    /// The function is debounced and can be called multiple times in quick
    /// succession.
    pub async fn mark_messages_as_read_debounced(
        &self,
        conversation_id: ConversationId,
        timestamp: String,
    ) -> Result<()> {
        let timestamp = timestamp.parse::<DateTime<Utc>>()?;
        self.app_state
            .mark_messages_read_debounced(conversation_id, timestamp)
            .await;
        Ok(())
    }

    /// This function is called from the flutter side to flush the debouncer
    /// state, immediately terminating the debouncer and marking all pending
    /// messages as read.
    pub async fn flush_debouncer_state(&self) -> Result<()> {
        self.app_state.flush_debouncer_state().await
    }

    /// Get the unread messages count across all conversations.
    pub async fn global_unread_messages_count(&self) -> u32 {
        self.user
            .global_unread_messages_count()
            .await
            .unwrap_or_default()
    }
}

pub(crate) fn group_messages(messages: Vec<ConversationMessage>) -> Vec<UiConversationMessage> {
    let mut grouped_messages = Vec::new();
    let mut iter = messages.into_iter().peekable();

    while let Some(conversation_message) = iter.next() {
        let mut timestamp = conversation_message.timestamp();
        let Message::Content(content_message) = conversation_message.message() else {
            // Directly add non-content messages
            grouped_messages.push(conversation_message.into());
            continue;
        };

        let mut current_flight = vec![content_message.clone().into()];
        let current_sender = content_message.sender().to_string();

        // Keep collecting messages from the same sender
        while let Some(next_message) = iter.peek() {
            let temp_timestamp = next_message.timestamp();
            let Message::Content(next_content_message) = next_message.message() else {
                break;
            };
            if next_content_message.sender() != current_sender {
                break;
            }
            let next_content_message = next_content_message.clone();
            // Consume the next message and add it to the current flight
            let _ = iter.next();
            timestamp = temp_timestamp;
            current_flight.push(next_content_message.into());
        }

        // Add the grouped messages to the result
        grouped_messages.push(UiConversationMessage {
            message: UiMessage::ContentFlight(current_flight),
            timestamp: timestamp.to_rfc3339(),
            ..conversation_message.into()
        });
    }

    grouped_messages
}

/// An internal broadcast channel for fetched messages
///
/// Must not be exposed to the UI.
#[derive(Debug, Clone)]
pub(crate) struct FetchedMessagesBroadcast {
    tx: broadcast::Sender<Arc<FetchedMessages>>,
}

impl FetchedMessagesBroadcast {
    pub(crate) fn new() -> Self {
        Self {
            tx: broadcast::channel(128).0,
        }
    }

    pub(crate) async fn send(&self, fetched_messages: FetchedMessages) {
        let _no_receivers = self.tx.send(Arc::new(fetched_messages));
    }

    pub(crate) fn subscribe(&self) -> FetchedMessagesReceiver {
        let rx = self.tx.subscribe();
        FetchedMessagesReceiver { rx }
    }
}

/// An internal received channel for fetched messages
///
/// Must not be exposed to the UI.
pub(crate) struct FetchedMessagesReceiver {
    rx: broadcast::Receiver<Arc<FetchedMessages>>,
}

impl FetchedMessagesReceiver {
    pub(crate) async fn recv(
        &mut self,
    ) -> Result<Arc<FetchedMessages>, broadcast::error::RecvError> {
        self.rx.recv().await
    }
}
