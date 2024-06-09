// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxcoreclient::{clients::process::ProcessQsMessageResult, MimiContent};
use phnxtypes::time::TimeStamp;

use super::{
    types::{ConversationIdBytes, UiConversationMessage},
    user::creation::User,
};

impl User {
    #[tokio::main(flavor = "current_thread")]
    pub async fn fetch_messages(&self) -> Result<()> {
        let mut user = self.user.lock().await;

        // Fetch AS messages
        let as_messages = user.as_fetch_messages().await?;

        // Process each as message individually and dispatch conversation
        // notifications to the UI in case a new conversation is created.
        let mut new_connections = vec![];
        for as_message in as_messages {
            let as_message_plaintext = user.decrypt_as_queue_message(as_message)?;
            let conversation_id = user.process_as_message(as_message_plaintext).await?;
            // Let the UI know that there'a s new conversation
            self.dispatch_conversation_notifications(vec![conversation_id])
                .await;
            new_connections.push(conversation_id);
        }

        // Send a notification to the OS (desktop only), the UI deals with
        // mobile notifications
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        self.send_desktop_os_connection_notifications(&user, new_connections)?;

        // Fetch QS messages
        let qs_messages = user.qs_fetch_messages().await?;
        // Process each qs message individually and dispatch conversation message notifications
        let mut new_conversations = vec![];
        let mut changed_conversations = vec![];
        let mut new_messages = vec![];
        for qs_message in qs_messages {
            let qs_message_plaintext = user.decrypt_qs_queue_message(qs_message)?;
            match user.process_qs_message(qs_message_plaintext).await? {
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
            self.dispatch_message_notifications(new_messages.clone()),
            self.dispatch_conversation_notifications(new_conversations.clone()),
            self.dispatch_conversation_notifications(changed_conversations.clone()),
        );

        // Send a notification to the OS (desktop only)
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.send_desktop_os_message_notifications(&user, new_messages)?;
            self.send_desktop_os_conversation_notifications(&user, new_conversations.clone())?;
        }

        // Update user auth keys of newly created conversations.
        let mut new_messages = vec![];
        for conversation_id in new_conversations {
            let messages = user.update_user_key(conversation_id).await?;
            new_messages.extend(messages);
        }

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.send_desktop_os_message_notifications(&user, new_messages)?;
        }

        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn send_message(
        &self,
        conversation_id: ConversationIdBytes,
        message: String,
    ) -> Result<UiConversationMessage> {
        let mut user = self.user.lock().await;
        let content = MimiContent::simple_markdown_message(user.user_name().domain(), message);
        user.send_message(conversation_id.into(), content)
            .await
            .map(|m| m.into())
    }

    pub async fn get_messages(
        &self,
        conversation_id: ConversationIdBytes,
        last_n: u32,
    ) -> Vec<UiConversationMessage> {
        let user = self.user.lock().await;
        user.get_messages(conversation_id.into(), last_n as usize)
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
