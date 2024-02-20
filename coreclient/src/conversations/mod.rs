// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls::group::GroupId;
use phnxtypes::{
    identifiers::{QualifiedGroupId, UserName},
    time::TimeStamp,
};
use serde::{Deserialize, Serialize};
use tls_codec::DeserializeBytes;
use uuid::Uuid;

use crate::utils::persistence::SqlKey;

pub(crate) mod messages;
pub(crate) mod store;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ConversationId {
    uuid: Uuid,
}

impl SqlKey for ConversationId {
    fn to_sql_key(&self) -> String {
        self.uuid.to_string()
    }
}

impl ConversationId {
    pub fn as_uuid(&self) -> Uuid {
        self.uuid
    }
}

impl From<Uuid> for ConversationId {
    fn from(uuid: Uuid) -> Self {
        Self { uuid }
    }
}

impl TryFrom<GroupId> for ConversationId {
    type Error = tls_codec::Error;

    fn try_from(value: GroupId) -> Result<Self, Self::Error> {
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(value.as_slice())?;
        let conversation_id = Self {
            uuid: Uuid::from_bytes(qgid.group_id),
        };
        Ok(conversation_id)
    }
}

impl SqlKey for GroupId {
    fn to_sql_key(&self) -> String {
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(self.as_slice()).unwrap();
        qgid.to_string()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub(super) struct ConversationPayload {
    status: ConversationStatus,
    conversation_type: ConversationType,
    attributes: ConversationAttributes,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Conversation {
    id: ConversationId,
    // Id of the (active) MLS group representing this conversation.
    group_id: GroupId,
    last_used: TimeStamp,
    // The timestamp of the last message that was (marked as) read by the user.
    last_read: TimeStamp,
    // Payload encoded as a byte array when communicating with the DB.
    pub(super) conversation_payload: ConversationPayload,
}

impl Conversation {
    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn conversation_type(&self) -> &ConversationType {
        &self.conversation_payload.conversation_type
    }

    pub fn status(&self) -> &ConversationStatus {
        &self.conversation_payload.status
    }

    pub fn attributes(&self) -> &ConversationAttributes {
        &self.conversation_payload.attributes
    }

    pub fn last_used(&self) -> &TimeStamp {
        &self.last_used
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ConversationStatus {
    Inactive(InactiveConversation),
    Active,
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct InactiveConversation {
    pub past_members: Vec<UserName>,
}

impl InactiveConversation {
    pub fn new(past_members: Vec<UserName>) -> Self {
        Self { past_members }
    }

    pub fn past_members(&self) -> &[UserName] {
        &self.past_members
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(UserName),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(UserName),
    Group,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationAttributes {
    title: String,
    conversation_picture_option: Option<Vec<u8>>,
}

impl ConversationAttributes {
    pub fn new(title: String, conversation_picture_option: Option<Vec<u8>>) -> Self {
        Self {
            title,
            conversation_picture_option,
        }
    }

    pub fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub fn conversation_picture_option(&self) -> Option<&[u8]> {
        self.conversation_picture_option
            .as_ref()
            .map(|v| v.as_slice())
    }

    pub fn set_conversation_picture_option(
        &mut self,
        conversation_picture_option: Option<Vec<u8>>,
    ) {
        self.conversation_picture_option = conversation_picture_option;
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
}
