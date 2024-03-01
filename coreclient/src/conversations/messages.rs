// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{groups::TimestampedMessage, mimi_content::MimiContent};

use super::*;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    // The ID is only globally consistent for actual content messages. Event
    // messages have local identifiers only.
    pub(super) conversation_id: ConversationId,
    local_message_id: Uuid,
    timestamped_message: TimestampedMessage,
}

impl ConversationMessage {
    /// Create a new conversation message from a group message. New messages are
    /// marked as unread by default.
    pub(crate) fn new(
        conversation_id: ConversationId,
        timestamped_message: TimestampedMessage,
    ) -> ConversationMessage {
        ConversationMessage {
            conversation_id,
            local_message_id: Uuid::new_v4(),
            timestamped_message,
        }
    }

    pub fn id_ref(&self) -> &Uuid {
        &self.local_message_id
    }

    pub fn id(&self) -> Uuid {
        self.local_message_id
    }

    pub fn timestamp(&self) -> TimeStamp {
        self.timestamped_message.ds_timestamp()
    }

    pub fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    pub fn message(&self) -> &Message {
        &self.timestamped_message.message()
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Content(ContentMessage),
    Display(DisplayMessage),
}

impl Message {
    /// Returns a string representation of the message for use in UI
    /// notifications.
    pub fn string_representation(&self, conversation_type: &ConversationType) -> String {
        match self {
            Message::Content(content_message) => match conversation_type {
                ConversationType::Group => {
                    let sender = &content_message.sender;
                    let content = content_message.content.string_rendering();
                    format!("{sender}: {content}")
                }
                ConversationType::Connection(_) | ConversationType::UnconfirmedConnection(_) => {
                    let content = content_message.content.string_rendering();
                    format!("{content}")
                }
            },
            Message::Display(display) => match &display.message {
                DisplayMessageType::System(system) => system.message().to_string(),
                DisplayMessageType::Error(error) => error.message().to_string(),
            },
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ContentMessage {
    sender: String,
    content: MimiContent,
}

impl ContentMessage {
    pub fn new(sender: String, content: MimiContent) -> Self {
        Self { sender, content }
    }

    pub fn sender(&self) -> &str {
        self.sender.as_ref()
    }

    pub fn content(&self) -> &MimiContent {
        &self.content
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMessage {
    message: DisplayMessageType,
}

impl DisplayMessage {
    pub(crate) fn new(message: DisplayMessageType) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &DisplayMessageType {
        &self.message
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DisplayMessageType {
    System(SystemMessage),
    Error(ErrorMessage),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    message: String,
}

impl SystemMessage {
    pub(crate) fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        self.message.as_ref()
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    message: String,
}

impl ErrorMessage {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        self.message.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct NotificationsRequest {}

#[derive(Debug, Clone)]
pub enum NotificationType {
    ConversationChange(ConversationId), // The id of the changed conversation.
    Message(ConversationMessage),
}
