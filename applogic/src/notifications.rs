// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::BTreeMap, fmt, str::FromStr};

use phnxcoreclient::{ConversationId, ConversationMessage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::user::User;

#[derive(Debug)]
pub(crate) struct NotificationId {
    conversation_id: ConversationId,
    notification_id: Uuid,
}

impl NotificationId {
    pub(crate) fn new(conversation_id: ConversationId) -> Self {
        Self {
            conversation_id,
            notification_id: Uuid::new_v4(),
        }
    }

    pub(crate) fn belongs_to(self, conversation_id: ConversationId) -> bool {
        self.conversation_id == conversation_id
    }

    #[cfg(any(target_os = "ios", target_os = "android"))]
    pub(crate) fn invalid() -> Self {
        Self {
            conversation_id: ConversationId::new(Uuid::nil()),
            notification_id: Uuid::nil(),
        }
    }

    pub(crate) fn into_conversation_id(self) -> ConversationId {
        self.conversation_id
    }
}

impl fmt::Display for NotificationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "im.phnx.prototype.conv:{}.id:{}",
            self.conversation_id, self.notification_id
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum NotificationIdParseError {
    #[error("missing conversation id")]
    MissingConversationId,
    #[error("missing conversation id")]
    MissingNotificationId,
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
}

impl FromStr for NotificationId {
    type Err = NotificationIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use NotificationIdParseError::*;

        let mut parts = s.rsplit('.');
        let notification_id = parts
            .next()
            .ok_or(MissingConversationId)?
            .strip_prefix("id:")
            .ok_or(MissingNotificationId)?;
        let conversation_id = parts
            .next()
            .ok_or(MissingConversationId)?
            .strip_prefix("conv:")
            .ok_or(MissingConversationId)?;
        Ok(Self {
            conversation_id: ConversationId::new(conversation_id.parse()?),
            notification_id: notification_id.parse()?,
        })
    }
}

impl Serialize for NotificationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NotificationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

pub(crate) fn show_notifications(
    notifications: BTreeMap<ConversationId, Vec<LocalNotificationContent>>,
) {
    for (conversation_id, notifications) in notifications {
        for notification in notifications {
            ::notifications::send(::notifications::Notification {
                identifier: NotificationId::new(conversation_id).to_string(),
                title: notification.title,
                body: notification.body,
            });
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
