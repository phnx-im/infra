// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircoreclient::{ChatId, ChatType, ConversationMessage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{notifications::DartNotificationService, user::User};

impl User {
    /// Send notifications for new messages.
    pub(crate) async fn new_message_notifications(
        &self,
        conversation_messages: &[ConversationMessage],
        notifications: &mut Vec<NotificationContent>,
    ) {
        for conversation_message in conversation_messages {
            if let Some(conversation) = self
                .user
                .chat(&conversation_message.conversation_id())
                .await
            {
                let title = match conversation.chat_type() {
                    ChatType::Connection(user_id) => self
                        .user
                        .user_profile(user_id)
                        .await
                        .display_name
                        .to_string(),
                    ChatType::HandleConnection(handle) => handle.plaintext().to_owned(),
                    ChatType::Group => conversation.attributes().title().to_string(),
                };
                let body = conversation_message
                    .message()
                    .string_representation(&self.user, conversation.chat_type())
                    .await;
                notifications.push(NotificationContent {
                    identifier: NotificationId::random(),
                    title: title.to_owned(),
                    body: body.to_owned(),
                    conversation_id: Some(conversation.id()),
                });
            }
        }
    }

    /// Send notifications for new conversations.
    pub(crate) async fn new_conversation_notifications(
        &self,
        conversation_ids: &[ChatId],
        notifications: &mut Vec<NotificationContent>,
    ) {
        for conversation_id in conversation_ids {
            if let Some(conversation) = self.user.chat(conversation_id).await {
                let title = format!("You were added to {}", conversation.attributes().title());
                let body = "Say hi to everyone".to_owned();
                notifications.push(NotificationContent {
                    identifier: NotificationId::random(),
                    title: title.to_owned(),
                    body: body.to_owned(),
                    conversation_id: Some(*conversation_id),
                });
            }
        }
    }

    /// Send notifications for new connection requests.
    pub(crate) async fn new_connection_request_notifications(
        &self,
        connection_conversations: &[ChatId],
        notifications: &mut Vec<NotificationContent>,
    ) {
        for conversation_id in connection_conversations {
            if let Some(conversation) = self.user.chat(conversation_id).await
                && let ChatType::Connection(client_id) = conversation.chat_type()
            {
                let contact_name = self.user.user_profile(client_id).await.display_name;
                let title = format!("New connection with {contact_name}");
                let body = "Say hi".to_owned();

                notifications.push(NotificationContent {
                    identifier: NotificationId::random(),
                    title,
                    body,
                    conversation_id: Some(*conversation_id),
                });
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationId(pub Uuid);

impl NotificationId {
    pub(crate) fn random() -> Self {
        Self(Uuid::new_v4())
    }

    pub(crate) fn invalid() -> Self {
        Self(Uuid::nil())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationContent {
    pub identifier: NotificationId,
    pub title: String,
    pub body: String,
    pub conversation_id: Option<ChatId>,
}

#[derive(Debug)]
pub struct NotificationHandle {
    pub identifier: NotificationId,
    pub conversation_id: Option<ChatId>,
}

#[derive(Debug, Clone)]
pub(crate) struct NotificationService {
    #[cfg(any(target_os = "ios", target_os = "android", target_os = "macos"))]
    dart_service: DartNotificationService,
}

impl NotificationService {
    #[allow(unused_variables)]
    pub(crate) fn new(dart_service: DartNotificationService) -> Self {
        Self {
            #[cfg(any(target_os = "ios", target_os = "android", target_os = "macos"))]
            dart_service,
        }
    }

    pub(crate) async fn show_notification(&self, notification: NotificationContent) {
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "macos"))]
        self.dart_service.send_notification(notification).await;
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if let Err(error) = notify_rust::Notification::new()
                .summary(notification.title.as_str())
                .body(notification.body.as_str())
                .show()
            {
                tracing::error!(%error, "Failed to send desktop notification");
            }
        }
    }

    pub(crate) async fn get_active_notifications(&self) -> Vec<NotificationHandle> {
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "macos"))]
        {
            self.dart_service.get_active_notifications().await
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            Vec::new()
        }
    }

    #[allow(unused_variables)]
    pub(crate) async fn cancel_notifications(&self, identifiers: Vec<NotificationId>) {
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "macos"))]
        self.dart_service.cancel_notifications(identifiers).await;
    }
}
