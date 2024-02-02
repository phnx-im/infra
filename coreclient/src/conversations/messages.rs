// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::GroupMessage;

use super::*;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub conversation_id: ConversationId,
    pub id: Uuid,
    pub timestamp: TimeStamp,
    pub message: Message,
}

impl ConversationMessage {
    pub(crate) fn new(
        conversation_id: ConversationId,
        group_message: GroupMessage,
    ) -> ConversationMessage {
        let (id, timestamp, message) = group_message.into_parts();
        ConversationMessage {
            conversation_id,
            id: id.into(),
            timestamp,
            message,
        }
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
    pub fn string_representation(&self) -> String {
        match self {
            Message::Content(content) => match &content.content {
                MessageContentType::Text(text) => {
                    format!(
                        "{}: {}",
                        content.sender,
                        String::from_utf8_lossy(text.message())
                    )
                }
                MessageContentType::Knock(_) => String::from("Knock"),
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
    pub sender: String,
    pub content: MessageContentType,
}

#[derive(
    PartialEq, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
#[repr(u16)]
pub enum MessageContentType {
    Text(TextMessage),
    Knock(Knock),
}

#[derive(
    PartialEq, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub struct TextMessage {
    message: Vec<u8>,
}

impl TextMessage {
    pub fn new(message: Vec<u8>) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &[u8] {
        self.message.as_ref()
    }
}

#[derive(
    PartialEq, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub struct Knock {}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMessage {
    pub message: DisplayMessageType,
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
