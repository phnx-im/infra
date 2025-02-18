// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Display;

use chrono::{DateTime, Utc};
use openmls::group::GroupId;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedGroupId, QualifiedUserName},
    time::TimeStamp,
};
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    Connection, ToSql,
};
use serde::{Deserialize, Serialize};
use tls_codec::DeserializeBytes;
use tracing::error;
use uuid::Uuid;

use crate::store::StoreNotifier;

pub(crate) mod messages;
pub(crate) mod persistence;

/// Id of a conversation
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConversationId {
    pub uuid: Uuid,
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

impl TryFrom<&GroupId> for ConversationId {
    type Error = tls_codec::Error;

    fn try_from(value: &GroupId) -> Result<Self, Self::Error> {
        let qgid = QualifiedGroupId::try_from(value.clone())?;
        let conversation_id = Self {
            uuid: qgid.group_uuid(),
        };
        Ok(conversation_id)
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
    // The timestamp of the last message that was (marked as) read by the user.
    last_read: DateTime<Utc>,
    status: ConversationStatus,
    conversation_type: ConversationType,
    attributes: ConversationAttributes,
}

impl Conversation {
    pub(crate) fn new_connection_conversation(
        group_id: GroupId,
        user_name: QualifiedUserName,
        attributes: ConversationAttributes,
    ) -> Result<Self, tls_codec::Error> {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let conversation = Conversation {
            id: ConversationId::try_from(&group_id)?,
            group_id,
            last_read: Utc::now(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_name),
            attributes,
        };
        Ok(conversation)
    }

    pub(crate) fn new_group_conversation(
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Self {
        let id = ConversationId::try_from(&group_id).unwrap();
        Self {
            id,
            group_id,
            last_read: Utc::now(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            attributes,
        }
    }

    pub fn id(&self) -> ConversationId {
        self.id
    }

    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn conversation_type(&self) -> &ConversationType {
        &self.conversation_type
    }

    pub fn status(&self) -> &ConversationStatus {
        &self.status
    }

    pub fn attributes(&self) -> &ConversationAttributes {
        &self.attributes
    }

    pub fn last_read(&self) -> DateTime<Utc> {
        self.last_read
    }

    pub(crate) fn owner_domain(&self) -> Fqdn {
        let qgid = QualifiedGroupId::try_from(self.group_id.clone()).unwrap();
        qgid.owning_domain().clone()
    }

    pub(crate) fn set_conversation_picture(
        &mut self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
        conversation_picture: Option<Vec<u8>>,
    ) -> Result<(), rusqlite::Error> {
        Self::update_picture(
            connection,
            notifier,
            self.id,
            conversation_picture.as_deref(),
        )?;
        self.attributes.set_picture(conversation_picture);
        Ok(())
    }

    pub(crate) fn set_inactive(
        &mut self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
        past_members: Vec<QualifiedUserName>,
    ) -> Result<(), rusqlite::Error> {
        let new_status = ConversationStatus::Inactive(InactiveConversation { past_members });
        Self::update_status(connection, notifier, self.id, &new_status)?;
        self.status = new_status;
        Ok(())
    }

    /// Confirm a connection conversation by setting the conversation type to
    /// `Connection`.
    pub(crate) fn confirm(
        &mut self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        if let ConversationType::UnconfirmedConnection(user_name) = self.conversation_type.clone() {
            let conversation_type = ConversationType::Connection(user_name);
            self.set_conversation_type(connection, notifier, &conversation_type)?;
            self.conversation_type = conversation_type;
        }
        Ok(())
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
        let user_names: Result<Vec<QualifiedUserName>, _> =
            user_names.split(',').map(|s| s.parse()).collect();
        let user_names = user_names.map_err(|error| {
            error!(%error, "Failed to parse user names from database");
            FromSqlError::Other(Box::new(error))
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
    pub past_members: Vec<QualifiedUserName>,
}

impl InactiveConversation {
    pub fn new(past_members: Vec<QualifiedUserName>) -> Self {
        Self { past_members }
    }

    pub fn past_members(&self) -> &[QualifiedUserName] {
        &self.past_members
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(QualifiedUserName),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(QualifiedUserName),
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
                user_name.parse().map_err(|error| {
                    error!(%error, "Failed to parse user name from database");
                    FromSqlError::Other(Box::new(error))
                })?,
            )),
            "connection" => Ok(Self::Connection(user_name.parse().map_err(|error| {
                error!(%error, "Failed to parse user name from database");
                FromSqlError::Other(Box::new(error))
            })?)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for ConversationType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let conversation_type = match self {
            Self::UnconfirmedConnection(user_name) => {
                format!("unconfirmed_connection:{}", user_name)
            }
            Self::Connection(user_name) => format!("connection:{}", user_name),
            Self::Group => "group".to_string(),
        };
        Ok(ToSqlOutput::Owned(Value::Text(conversation_type)))
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationAttributes {
    title: String,
    picture: Option<Vec<u8>>,
}

impl ConversationAttributes {
    pub fn new(title: String, picture: Option<Vec<u8>>) -> Self {
        Self { title, picture }
    }

    pub fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn picture(&self) -> Option<&[u8]> {
        self.picture.as_deref()
    }

    pub fn set_picture(&mut self, picture: Option<Vec<u8>>) {
        self.picture = picture;
    }
}
