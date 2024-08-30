// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use chrono::{DateTime, Utc};
use phnxcoreclient::{
    clients::process::ProcessQsMessageResult, ConversationId, ConversationMessage, MimiContent,
};

use crate::notifier::{dispatch_conversation_notifications, dispatch_message_notifications};

use super::{
    notifications::LocalNotificationContent,
    types::{ConversationIdBytes, UiConversationMessage},
    user::User,
};

pub(crate) struct FetchedMessages {
    pub(crate) new_conversations: Vec<ConversationId>,
    pub(crate) changed_conversations: Vec<ConversationId>,
    pub(crate) new_messages: Vec<ConversationMessage>,
    pub(crate) notifications_content: Vec<LocalNotificationContent>,
}

pub(crate) struct FetchedQsMessages {
    pub(crate) new_conversations: Vec<ConversationId>,
    pub(crate) changed_conversations: Vec<ConversationId>,
    pub(crate) new_messages: Vec<ConversationMessage>,
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
    pub(crate) async fn fetch_qs_messages(&self) -> Result<FetchedQsMessages> {
        let qs_messages = self.user.qs_fetch_messages().await?;
        // Process each qs message individually and dispatch conversation message notifications
        let mut new_conversations = vec![];
        let mut changed_conversations = vec![];
        let mut new_messages = vec![];
        for qs_message in qs_messages {
            let qs_message_plaintext = self.user.decrypt_qs_queue_message(qs_message).await?;
            match self.user.process_qs_message(qs_message_plaintext).await? {
                ProcessQsMessageResult::ConversationMessages(conversation_messages) => {
                    new_messages.extend(conversation_messages);
                }
                ProcessQsMessageResult::ConversationChanged(
                    conversation_id,
                    conversation_messages,
                ) => {
                    new_messages.extend(conversation_messages);
                    changed_conversations.push(conversation_id)
                }
                ProcessQsMessageResult::NewConversation(conversation_id) => {
                    new_conversations.push(conversation_id)
                }
            };
        }

        // Update user auth keys of newly created conversations.
        for conversation_id in &new_conversations {
            let messages = self.user.update_user_key(conversation_id).await?;
            new_messages.extend(messages);
        }

        Ok(FetchedQsMessages {
            new_conversations,
            changed_conversations,
            new_messages,
        })
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
        let FetchedQsMessages {
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
        conversation_id: ConversationIdBytes,
        message: String,
    ) -> Result<UiConversationMessage> {
        let content = MimiContent::simple_markdown_message(self.user.user_name().domain(), message);
        self.user
            .send_message(conversation_id.into(), content)
            .await
            .map(|m| m.into())
    }

    pub async fn get_messages(
        &self,
        conversation_id: ConversationIdBytes,
        last_n: u32,
    ) -> Vec<UiConversationMessage> {
        self.user
            .get_messages(conversation_id.into(), last_n as usize)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.into())
            .collect()
    }

    /// This function is called from the flutter side to mark messages as read.
    ///
    /// The function is debounced and can be called multiple times in quick
    /// succession.
    pub async fn mark_messages_as_read_debounced(
        &self,
        conversation_id: ConversationIdBytes,
        timestamp: String,
    ) -> Result<()> {
        let timestamp = timestamp.parse::<DateTime<Utc>>()?;
        self.app_state
            .mark_messages_read_debounced(conversation_id.into(), timestamp.into())
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
