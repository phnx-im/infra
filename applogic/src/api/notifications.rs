// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) use phnxcoreclient::{ConversationId, ConversationMessage};

use crate::api::user::User;

pub(crate) struct LocalNotificationContent {
    pub(crate) title: String,
    pub(crate) body: String,
}

impl User {
    /// Send notifications for new messages.
    pub(crate) async fn new_message_notifications(
        &self,
        conversation_messages: &[ConversationMessage],
    ) -> Vec<LocalNotificationContent> {
        let mut notifications = Vec::new();

        for conversation_message in conversation_messages {
            if let Some(conversation) = self
                .user
                .conversation(conversation_message.conversation_id())
                .await
            {
                let title = match conversation.conversation_type() {
                    phnxcoreclient::ConversationType::UnconfirmedConnection(username)
                    | phnxcoreclient::ConversationType::Connection(username) => {
                        username.to_string()
                    }
                    phnxcoreclient::ConversationType::Group => {
                        conversation.attributes().title().to_string()
                    }
                };
                let body = conversation_message
                    .message()
                    .string_representation(conversation.conversation_type());
                notifications.push(LocalNotificationContent {
                    title: title.to_owned(),
                    body: body.to_owned(),
                });
            }
        }

        notifications
    }

    /// Send notifications for new conversations.
    pub(crate) async fn new_conversation_notifications(
        &self,
        conversation_ids: &[ConversationId],
    ) -> Vec<LocalNotificationContent> {
        let mut notifications = Vec::new();

        for conversation_id in conversation_ids {
            if let Some(conversation) = self.user.conversation(conversation_id).await {
                let title = format!("You were added to {}", conversation.attributes().title());
                let body = "Say hi to everyone".to_owned();
                notifications.push(LocalNotificationContent {
                    title: title.to_owned(),
                    body: body.to_owned(),
                });
            }
        }

        notifications
    }

    /// Send notifications for new connection requests.
    pub(crate) async fn new_connection_request_notifications(
        &self,
        connection_conversations: &[ConversationId],
    ) -> Vec<LocalNotificationContent> {
        let mut notifications = Vec::new();

        for conversation_id in connection_conversations {
            if let Some(conversation) = self.user.conversation(conversation_id).await {
                let contact_name = match conversation.conversation_type() {
                    phnxcoreclient::ConversationType::UnconfirmedConnection(username)
                    | phnxcoreclient::ConversationType::Connection(username) => {
                        username.to_string()
                    }
                    _ => "".to_string(),
                };
                let title = format!("New connection request from {}", contact_name);
                let body = "Open to accept or ignore".to_owned();
                notifications.push(LocalNotificationContent {
                    title: title.to_owned(),
                    body: body.to_owned(),
                });
            }
        }

        notifications
    }
}
