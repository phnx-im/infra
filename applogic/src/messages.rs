// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::messages::QueueMessage;
use aircoreclient::{ChatId, clients::process::process_qs::ProcessedQsMessages};
use anyhow::Result;
use tracing::debug;

use crate::{api::user::User, notifications::NotificationContent};

#[derive(Debug, Default)]
pub(crate) struct ProcessedMessages {
    pub(crate) notifications_content: Vec<NotificationContent>,
}

impl User {
    /// Fetch and process AS messages
    async fn fetch_and_process_as_messages(&self) -> Result<Vec<ChatId>> {
        self.user.fetch_and_process_handle_messages().await
    }

    /// Fetch and process QS messages
    async fn fetch_and_process_qs_messages(&self) -> Result<ProcessedQsMessages> {
        let qs_messages = self.user.qs_fetch_messages().await?;
        self.user.fully_process_qs_messages(qs_messages).await
    }

    pub(crate) async fn process_qs_messages(
        &self,
        qs_messages: Vec<QueueMessage>,
    ) -> Result<ProcessedQsMessages> {
        self.user.fully_process_qs_messages(qs_messages).await
    }

    /// Fetch and process both QS and AS messages
    ///
    /// This function is intended to be called in the background service.
    pub(crate) async fn fetch_and_process_all_messages_in_background(
        &self,
    ) -> Result<ProcessedMessages> {
        let mut notifications = Vec::new();

        // Fetch QS messages
        debug!("fetch QS messages");
        let ProcessedQsMessages {
            new_chats,
            changed_chats: _,
            new_messages,
            errors: _,
        } = self.fetch_and_process_qs_messages().await?;
        self.new_chat_notifications(&new_chats, &mut notifications)
            .await;
        self.new_message_notifications(&new_messages, &mut notifications)
            .await;

        // Fetch AS connection requests
        debug!("fetch AS messages");
        let new_connections = self.fetch_and_process_as_messages().await?;
        self.new_connection_request_notifications(&new_connections, &mut notifications)
            .await;

        Ok(ProcessedMessages {
            notifications_content: notifications,
        })
    }
}
