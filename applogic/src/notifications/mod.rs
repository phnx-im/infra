// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

use phnxcoreclient::{ConversationId, ConversationMessage};

use crate::api::user::User;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) fn init_desktop_os_notifications() -> Result<(), notify_rust::error::Error> {
    #[cfg(target_os = "macos")]
    {
        let res = notify_rust::set_application("im.phnx.prototype");
        if res.is_err() {
            tracing::warn!("Could not set application for desktop notifications");
        }
    }

    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) fn show_desktop_notifications<'a>(
    notifications: impl Iterator<Item = &'a LocalNotificationContent>,
) {
    for notification in notifications {
        if let Err(error) = notify_rust::Notification::new()
            .summary(notification.title.as_str())
            .body(notification.body.as_str())
            .show()
        {
            tracing::error!(%error, "Failed to send desktop notification");
        }
    }
}

#[derive(Debug)]
pub(crate) struct LocalNotificationContent {
    pub(crate) title: String,
    pub(crate) body: String,
}

impl User {
    /// Send notifications for new messages.
    pub(crate) async fn new_message_notifications(
        &self,
        conversation_messages: &[ConversationMessage],
        notifications: &mut BTreeMap<ConversationId, Vec<LocalNotificationContent>>,
    ) {
        for conversation_message in conversation_messages {
            if let Some(conversation) = self
                .user
                .conversation(&conversation_message.conversation_id())
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
                notifications.entry(conversation.id()).or_default().push(
                    LocalNotificationContent {
                        title: title.to_owned(),
                        body: body.to_owned(),
                    },
                );
            }
        }
    }

    /// Send notifications for new conversations.
    pub(crate) async fn new_conversation_notifications(
        &self,
        conversation_ids: &[ConversationId],
        notifications: &mut BTreeMap<ConversationId, Vec<LocalNotificationContent>>,
    ) {
        for conversation_id in conversation_ids {
            if let Some(conversation) = self.user.conversation(conversation_id).await {
                let title = format!("You were added to {}", conversation.attributes().title());
                let body = "Say hi to everyone".to_owned();
                notifications
                    .entry(*conversation_id)
                    .or_default()
                    .push(LocalNotificationContent {
                        title: title.to_owned(),
                        body: body.to_owned(),
                    });
            }
        }
    }

    /// Send notifications for new connection requests.
    pub(crate) async fn new_connection_request_notifications(
        &self,
        connection_conversations: &[ConversationId],
        notifications: &mut BTreeMap<ConversationId, Vec<LocalNotificationContent>>,
    ) {
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

                notifications
                    .entry(*conversation_id)
                    .or_default()
                    .push(LocalNotificationContent {
                        title: title.to_owned(),
                        body: body.to_owned(),
                    });
            }
        }
    }
}
