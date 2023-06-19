// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::prelude::GroupId;
use phnxbackend::auth_service::UserName;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UuidBytes {
    pub bytes: [u8; 16],
}

impl UuidBytes {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes.copy_from_slice(bytes);
        Self { bytes: uuid_bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_uuid(uuid: &Uuid) -> Self {
        Self {
            bytes: *uuid.as_bytes(),
        }
    }

    pub fn as_uuid(&self) -> Uuid {
        Uuid::from_bytes(self.bytes)
    }
}

#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub id: UuidBytes,
    pub timestamp: u64,
    pub message: Message,
}

#[derive(Debug, Clone)]
pub enum Message {
    Content(ContentMessage),
    Display(DisplayMessage),
}

#[derive(Debug, Clone)]
pub struct ContentMessage {
    pub sender: UserName,
    pub content: MessageContentType,
}

#[derive(Debug, Clone)]
pub enum MessageContentType {
    Text(TextMessage),
    Ping(Ping),
}

#[derive(Debug, Clone)]
pub struct TextMessage {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Ping {}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub message: DisplayMessageType,
}

#[derive(Debug, Clone)]
pub enum DisplayMessageType {
    System(SystemMessage),
    Error(ErrorMessage),
}

#[derive(Debug, Clone)]
pub struct SystemMessage {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ErrorMessage {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupId,
    pub status: ConversationStatus,
    pub conversation_type: ConversationType,
    pub last_used: u64,
    pub attributes: ConversationAttributes,
}

#[derive(Debug, Clone)]
pub enum ConversationStatus {
    Inactive(InactiveConversation),
    Active(ActiveConversation),
}

#[derive(Debug, Clone)]
pub struct InactiveConversation {
    pub past_members: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ActiveConversation {}

#[derive(Debug, Clone)]
pub enum ConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(UserName),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(UserName),
    Group,
}

#[derive(Debug, Clone)]
pub struct ConversationAttributes {
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct DispatchedConversationMessage {
    pub conversation_id: Uuid,
    pub conversation_message: ConversationMessage,
}

#[derive(Debug, Clone)]
pub struct NotificationsRequest {}

#[derive(Debug, Clone)]
pub enum NotificationType {
    ConversationChange,
    Message(DispatchedConversationMessage),
}
