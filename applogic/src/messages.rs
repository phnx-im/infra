// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxcoreclient::{ConversationId, clients::process::process_qs::ProcessedQsMessages};
use tracing::debug;

use crate::{api::user::User, notifications::NotificationContent};

#[derive(Debug, Default)]
pub(crate) struct FetchedMessages {
    pub(crate) notifications_content: Vec<NotificationContent>,
}

impl User {
    /// Fetch AS messages
    async fn fetch_as_messages(&self) -> Result<Vec<ConversationId>> {
        let as_messages = self.user.as_fetch_messages().await?;

        // Process each as message individually and dispatch conversation
        // notifications to the UI in case a new conversation is created.
        let mut new_connections = vec![];
        for as_message in as_messages {
            let as_message_plaintext = match self.user.decrypt_as_queue_message(as_message).await {
                Ok(plaintext) => plaintext,
                Err(error) => {
                    tracing::error!(%error, "Failed to decrypt AS message; skipping");
                    continue;
                }
            };
            let conversation_id = self.user.process_as_message(as_message_plaintext).await?;
            new_connections.push(conversation_id);
        }

        Ok(new_connections)
    }

    /// Fetch QS messages
    async fn fetch_qs_messages(&self) -> Result<ProcessedQsMessages> {
        let qs_messages = self.user.qs_fetch_messages().await?;
        self.user.fully_process_qs_messages(qs_messages).await
    }

    /// Fetch both AS and QS messages
    pub(crate) async fn fetch_all_messages(&self) -> Result<FetchedMessages> {
        let mut notifications = Vec::new();

        // Fetch AS connection requests
        debug!("fetch AS messages");
        let new_connections = self.fetch_as_messages().await?;
        self.new_connection_request_notifications(&new_connections, &mut notifications)
            .await;

        // Fetch QS messages
        debug!("fetch QS messages");
        let ProcessedQsMessages {
            new_conversations,
            changed_conversations: _,
            new_messages,
            errors: _,
        } = self.fetch_qs_messages().await?;
        self.new_conversation_notifications(&new_conversations, &mut notifications)
            .await;
        self.new_message_notifications(&new_messages, &mut notifications)
            .await;

        Ok(FetchedMessages {
            notifications_content: notifications,
        })
    }
}
