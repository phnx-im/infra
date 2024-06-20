// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxcoreclient::{clients::process::ProcessQsMessageResult, MimiContent};
use phnxtypes::time::TimeStamp;

use crate::notifications::{
    dispatch_conversation_notifications, dispatch_message_notifications,
    send_desktop_os_connection_notifications, send_desktop_os_conversation_notifications,
    send_desktop_os_message_notifications,
};

use super::{
    types::{ConversationIdBytes, UiConversationMessage},
    user::User,
};

impl User {
    #[tokio::main(flavor = "current_thread")]
    pub async fn fetch_messages(&self) -> Result<()> {
        // Fetch AS messages
        let as_messages = self.user.as_fetch_messages().await?;

        // Process each as message individually and dispatch conversation
        // notifications to the UI in case a new conversation is created.
        let mut new_connections = vec![];
        for as_message in as_messages {
            let as_message_plaintext = self.user.decrypt_as_queue_message(as_message).await?;
            let conversation_id = self.user.process_as_message(as_message_plaintext).await?;
            // Let the UI know that there'a s new conversation
            dispatch_conversation_notifications(&self.notification_hub, vec![conversation_id])
                .await;
            new_connections.push(conversation_id);
        }

        // Send a notification to the OS (desktop only), the UI deals with
        // mobile notifications
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        send_desktop_os_connection_notifications(&self.user, new_connections).await?;

        // Fetch QS messages
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

        // Let the UI know there is new stuff
        tokio::join!(
            dispatch_message_notifications(&self.notification_hub, new_messages.clone()),
            dispatch_conversation_notifications(&self.notification_hub, new_conversations.clone()),
            dispatch_conversation_notifications(
                &self.notification_hub,
                changed_conversations.clone()
            ),
        );

        // Send a notification to the OS (desktop only)
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            send_desktop_os_message_notifications(&self.user, new_messages).await?;
            send_desktop_os_conversation_notifications(&self.user, new_conversations.clone())
                .await?;
        }

        // Update user auth keys of newly created conversations.
        let mut new_messages = vec![];
        for conversation_id in new_conversations {
            let messages = self.user.update_user_key(conversation_id).await?;
            new_messages.extend(messages);
        }

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            send_desktop_os_message_notifications(&self.user, new_messages).await?;
        }

        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
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
        timestamp: u64,
    ) -> Result<()> {
        let timestamp = TimeStamp::try_from(timestamp)?;
        self.app_state
            .mark_messages_read_debounced(conversation_id.into(), timestamp)
            .await;
        Ok(())
    }

    /// This function is called from the flutter side to flush the debouncer
    /// state, immediately terminating the debouncer and marking all pending
    /// messages as read.
    #[tokio::main(flavor = "current_thread")]
    pub async fn flush_debouncer_state(&self) -> Result<()> {
        self.app_state.flush_debouncer_state().await
    }
}
