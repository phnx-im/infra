// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::prelude::GroupId;
use phnxtypes::identifiers::{QualifiedGroupId, UserName};
use serde::{Deserialize, Serialize};
use tls_codec::{DeserializeBytes, TlsDeserialize, TlsSerialize, TlsSize};
use uuid::Uuid;

use crate::groups::GroupMessage;

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct GroupIdBytes {
    pub bytes: Vec<u8>,
}

impl std::fmt::Display for GroupIdBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let group_id = self.as_group_id();
        let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice())
            .map_err(|_| std::fmt::Error)?;

        write!(f, "{}", qgid)
    }
}

impl From<GroupId> for GroupIdBytes {
    fn from(group_id: GroupId) -> Self {
        Self {
            bytes: group_id.as_slice().to_vec(),
        }
    }
}

impl GroupIdBytes {
    pub fn as_group_id(&self) -> GroupId {
        GroupId::from_slice(&self.bytes)
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct UuidBytes {
    pub bytes: [u8; 16],
}

impl From<Uuid> for UuidBytes {
    fn from(uuid: Uuid) -> Self {
        Self {
            bytes: *uuid.as_bytes(),
        }
    }
}

impl std::fmt::Display for UuidBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uuid = self.as_uuid();
        write!(f, "{}", uuid)
    }
}

impl UuidBytes {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes.copy_from_slice(bytes);
        Self { bytes: uuid_bytes }
    }

    pub fn from_group_id(group_id: &GroupId) -> Self {
        let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice()).unwrap();
        Self {
            bytes: qgid.group_id,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            bytes: *uuid.as_bytes(),
        }
    }

    pub fn as_uuid(&self) -> Uuid {
        Uuid::from_bytes(self.bytes.clone().try_into().unwrap())
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub conversation_id: UuidBytes,
    pub id: UuidBytes,
    pub timestamp: u64,
    pub message: Message,
}

impl ConversationMessage {
    pub(crate) fn new(conversation_id: Uuid, group_message: GroupMessage) -> ConversationMessage {
        let (id, timestamp, message) = group_message.into_parts();
        ConversationMessage {
            conversation_id: UuidBytes::from_uuid(conversation_id),
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
    PartialEq, Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize,
)]
#[repr(u16)]
pub enum MessageContentType {
    Text(TextMessage),
    Knock(Knock),
}

#[derive(
    PartialEq, Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize,
)]
pub struct TextMessage {
    pub message: Vec<u8>,
}

#[derive(
    PartialEq, Debug, Clone, TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize,
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
    pub message: String,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Conversation {
    pub id: UuidBytes,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupIdBytes,
    pub status: ConversationStatus,
    pub conversation_type: ConversationType,
    pub last_used: u64,
    pub attributes: ConversationAttributes,
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ConversationStatus {
    Inactive(InactiveConversation),
    Active,
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct InactiveConversation {
    pub past_members: Vec<String>,
}

impl InactiveConversation {
    pub fn new(past_members: Vec<UserName>) -> Self {
        Self {
            past_members: past_members
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        }
    }

    pub fn past_members(&self) -> Vec<UserName> {
        self.past_members
            .iter()
            .map(|s| UserName::from(s.clone()))
            .collect()
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(String),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(String),
    Group,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationAttributes {
    pub title: String,
}

#[derive(PartialEq, Debug, Clone)]
pub struct DispatchedConversationMessage {
    pub conversation_id: UuidBytes,
    pub conversation_message: ConversationMessage,
}

#[derive(Debug, Clone)]
pub struct NotificationsRequest {}

#[derive(Debug, Clone)]
pub enum NotificationType {
    ConversationChange(UuidBytes), // The id of the changed conversation.
    Message(DispatchedConversationMessage),
}
