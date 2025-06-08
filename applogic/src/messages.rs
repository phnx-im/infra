// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use phnxcoreclient::{ConversationId, clients::process::process_qs::ProcessedQsMessages};
use tracing::{debug, error, warn};

use crate::{api::user::User, notifications::NotificationContent};

#[derive(Debug, Default)]
pub(crate) struct FetchedMessages {
    pub(crate) notifications_content: Vec<NotificationContent>,
}

impl User {
    /// Fetch AS messages
    async fn fetch_as_messages(&self) -> Result<Vec<ConversationId>> {
        let as_messages = self.user.as_fetch_messages().await?;
        if !as_messages.is_empty() {
            error!(num_messages = as_messages.len(), "ignoring AS messages");
        }
        Ok(Vec::new())
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
