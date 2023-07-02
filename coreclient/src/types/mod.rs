// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::prelude::GroupId;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
//use phnxbackend::auth_service::UserName;
use uuid::Uuid;

#[derive(PartialEq, Debug, Clone)]
pub struct UuidBytes {
    pub bytes: [u8; 16],
}

impl UuidBytes {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes.copy_from_slice(bytes);
        Self { bytes: uuid_bytes }
    }

    pub fn from_group_id(group_id: &GroupId) -> Self {
        Self::from_bytes(group_id.as_slice())
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

    pub fn as_group_id(&self) -> GroupId {
        GroupId::from_slice(&self.bytes)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct ConversationMessage {
    pub id: UuidBytes,
    pub timestamp: u64,
    pub message: Message,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Message {
    Content(ContentMessage),
    Display(DisplayMessage),
}

#[derive(PartialEq, Debug, Clone)]
pub struct ContentMessage {
    pub sender: Vec<u8>,
    pub content: MessageContentType,
}

#[derive(PartialEq, Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
#[repr(u16)]
pub enum MessageContentType {
    Text(TextMessage),
    Knock(Knock),
}

#[derive(PartialEq, Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct TextMessage {
    pub message: Vec<u8>,
}

#[derive(PartialEq, Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct Knock {}

#[derive(PartialEq, Debug, Clone)]
pub struct DisplayMessage {
    pub message: DisplayMessageType,
}

#[derive(PartialEq, Debug, Clone)]
pub enum DisplayMessageType {
    System(SystemMessage),
    Error(ErrorMessage),
}

#[derive(PartialEq, Debug, Clone)]
pub struct SystemMessage {
    pub message: String,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ErrorMessage {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: UuidBytes,
    pub status: ConversationStatus,
    pub conversation_type: ConversationType,
    pub last_used: u64,
    pub attributes: ConversationAttributes,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ConversationStatus {
    Inactive(InactiveConversation),
    Active,
}

#[derive(PartialEq, Debug, Clone)]
pub struct InactiveConversation {
    pub past_members: Vec<String>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(Vec<u8>),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(Vec<u8>),
    Group,
}

#[derive(Debug, Clone)]
pub struct ConversationAttributes {
    pub title: String,
}

#[derive(PartialEq, Debug, Clone)]
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
