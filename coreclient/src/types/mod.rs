// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use openmls::prelude::GroupId;
use phnxbackend::{
    auth_service::{AsClientId, UserName},
    ds::api::QualifiedGroupId,
    qs::Fqdn,
};
use serde::{Deserialize, Serialize};
use tls_codec::{
    DeserializeBytes, Serialize as TlsSerializeTrait, TlsDeserialize, TlsSerialize, TlsSize,
};
use uuid::Uuid;

use crate::{
    groups::GroupMessage,
    utils::{persistence::Persistable, Timestamp},
};

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct GroupIdBytes {
    pub bytes: Vec<u8>,
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
    pub rowid: Option<i64>,
    pub own_client_id: Vec<u8>,
    pub conversation_id: UuidBytes,
    pub id: UuidBytes,
    pub timestamp: u64,
    pub message: Message,
}

impl ConversationMessage {
    pub(crate) fn new(
        own_client_id: AsClientId,
        conversation_id: Uuid,
        group_message: GroupMessage,
    ) -> ConversationMessage {
        let (id, timestamp, message) = group_message.into_parts();
        ConversationMessage {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached().unwrap(),
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
    pub rowid: Option<i64>,
    pub own_client_id: Vec<u8>,
    pub id: UuidBytes,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupIdBytes,
    pub status: ConversationStatus,
    pub conversation_type: ConversationType,
    pub last_used: u64,
    pub attributes: ConversationAttributes,
}

impl Conversation {
    pub(crate) fn create_connection_conversation(
        own_client_id: &AsClientId,
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Result<Self> {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let uuid_bytes = UuidBytes::from_group_id(&group_id);
        let conversation = Conversation {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            id: uuid_bytes.clone(),
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_name.to_string()),
            last_used: Timestamp::now().as_u64(),
            attributes,
        };
        conversation.persist()?;
        Ok(conversation)
    }

    pub(crate) fn create_group_conversation(
        own_client_id: &AsClientId,
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Result<Self> {
        let uuid_bytes = UuidBytes::from_group_id(&group_id);
        let conversation = Conversation {
            rowid: None,
            own_client_id: own_client_id.tls_serialize_detached()?,
            id: uuid_bytes.clone(),
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            last_used: Timestamp::now().as_u64(),
            attributes,
        };
        conversation.persist()?;
        Ok(conversation)
    }

    pub(crate) fn owner_domain(&self) -> Fqdn {
        let qgid = QualifiedGroupId::tls_deserialize_exact(&self.group_id.bytes).unwrap();
        qgid.owning_domain
    }

    pub(crate) fn confirm(&mut self) -> Result<()> {
        if let ConversationType::UnconfirmedConnection(user_name) = self.conversation_type.clone() {
            self.conversation_type = ConversationType::Connection(user_name);
        }
        self.persist()?;
        Ok(())
    }

    pub(crate) fn set_inactive(&mut self, past_members: &[String]) -> Result<()> {
        self.status = ConversationStatus::Inactive(InactiveConversation {
            past_members: past_members.iter().map(|m| m.to_owned()).collect(),
        });
        self.persist()?;
        Ok(())
    }

    pub(crate) fn id(&self) -> Uuid {
        self.id.as_uuid()
    }
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
