// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Display;

use openmls::group::GroupId;
use phnxtypes::{
    identifiers::{QualifiedGroupId, SafeTryInto, UserName},
    time::TimeStamp,
};
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    ToSql,
};
use serde::{Deserialize, Serialize};
use tls_codec::DeserializeBytes;
use uuid::Uuid;

use crate::utils::persistence::SqlKey;

pub(crate) mod messages;
pub(crate) mod persistence;
pub(crate) mod store;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ConversationId {
    uuid: Uuid,
}

impl Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

impl ToSql for ConversationId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.uuid.to_sql()
    }
}

impl FromSql for ConversationId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let uuid = Uuid::column_result(value)?;
        Ok(Self { uuid })
    }
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

impl FromSql for ConversationStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let status = String::column_result(value)?;
        if status.starts_with("active") {
            return Ok(Self::Active);
        }
        let Some(user_names) = status.strip_prefix("inactive:") else {
            return Err(FromSqlError::InvalidType);
        };
        let user_names = user_names
            .split(',')
            .map(<&str as SafeTryInto<UserName>>::try_into)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                log::error!("Failed to parse user names from database: {:?}", e);
                FromSqlError::Other(Box::new(e))
            })?;
        Ok(Self::Inactive(InactiveConversation::new(user_names)))
    }
}

impl ToSql for ConversationStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let status = match self {
            Self::Active => "active".to_string(),
            Self::Inactive(inactive_conversation) => {
                let user_names = inactive_conversation
                    .past_members()
                    .iter()
                    .map(|user_name| user_name.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("inactive:{}", user_names)
            }
        };
        Ok(ToSqlOutput::Owned(Value::Text(status)))
    }
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

impl FromSql for ConversationType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let conversation_type = String::column_result(value)?;
        if conversation_type.starts_with("group") {
            return Ok(Self::Group);
        }
        let Some((conversation_type, user_name)) = conversation_type.split_once(':') else {
            return Err(FromSqlError::InvalidType);
        };
        match conversation_type {
            "unconfirmed_connection" => Ok(Self::UnconfirmedConnection(
                <&str as SafeTryInto<UserName>>::try_into(user_name).map_err(|e| {
                    log::error!("Failed to parse user name from database: {:?}", e);
                    FromSqlError::Other(Box::new(e))
                })?,
            )),
            "connection" => Ok(Self::Connection(
                <&str as SafeTryInto<UserName>>::try_into(user_name).map_err(|e| {
                    log::error!("Failed to parse user name from database: {:?}", e);
                    FromSqlError::Other(Box::new(e))
                })?,
            )),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for ConversationType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let conversation_type = match self {
            Self::UnconfirmedConnection(user_name) => {
                format!("unconfirmed_connection:{}", user_name.to_string())
            }
            Self::Connection(user_name) => format!("connection:{}", user_name.to_string()),
            Self::Group => "group".to_string(),
        };
        Ok(ToSqlOutput::Owned(Value::Text(conversation_type)))
    }
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
