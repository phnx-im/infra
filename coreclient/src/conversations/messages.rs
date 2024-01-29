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

#[derive(PartialEq, Debug, Clone)]
pub struct DispatchedConversationMessage {
    pub conversation_id: ConversationId,
    pub conversation_message: ConversationMessage,
}

impl From<ConversationMessage> for DispatchedConversationMessage {
    fn from(conversation_message: ConversationMessage) -> Self {
        Self {
            conversation_id: conversation_message.conversation_id.clone(),
            conversation_message,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationsRequest {}

#[derive(Debug, Clone)]
pub enum NotificationType {
    ConversationChange(ConversationId), // The id of the changed conversation.
    Message(DispatchedConversationMessage),
}
