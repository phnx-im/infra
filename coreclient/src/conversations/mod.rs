// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Display;

use chrono::{DateTime, Utc};
use openmls::group::GroupId;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedGroupId, UserId},
    time::TimeStamp,
};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqliteExecutor};
use uuid::Uuid;

use crate::store::StoreNotifier;

pub(crate) mod messages;
pub(crate) mod persistence;
mod sqlx_support;

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

impl ConversationId {
    pub fn random() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }

    pub fn new(uuid: Uuid) -> Self {
        Self { uuid }
    }

    pub fn uuid(&self) -> Uuid {
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
    pub id: ConversationId,
    // Id of the (active) MLS group representing this conversation.
    pub group_id: GroupId,
    // The timestamp of the last message that was (marked as) read by the user.
    pub last_read: DateTime<Utc>,
    pub status: ConversationStatus,
    pub conversation_type: ConversationType,
    pub attributes: ConversationAttributes,
}

impl Conversation {
    pub(crate) fn new_connection_conversation(
        group_id: GroupId,
        user_id: UserId,
        attributes: ConversationAttributes,
    ) -> Result<Self, tls_codec::Error> {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let conversation = Conversation {
            id: ConversationId::try_from(&group_id)?,
            group_id,
            last_read: Utc::now(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_id),
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

    pub fn status_mut(&mut self) -> &mut ConversationStatus {
        &mut self.status
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

    pub(crate) async fn set_conversation_picture(
        &mut self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_picture: Option<Vec<u8>>,
    ) -> sqlx::Result<()> {
        Self::update_picture(executor, notifier, self.id, conversation_picture.as_deref()).await?;
        self.attributes.set_picture(conversation_picture);
        Ok(())
    }

    pub(crate) async fn set_inactive(
        &mut self,
        executor: &mut SqliteConnection,
        notifier: &mut StoreNotifier,
        past_members: Vec<UserId>,
    ) -> sqlx::Result<()> {
        let new_status = ConversationStatus::Inactive(InactiveConversation { past_members });
        Self::update_status(executor, notifier, self.id, &new_status).await?;
        self.status = new_status;
        Ok(())
    }

    /// Confirm a connection conversation by setting the conversation type to
    /// `Connection`.
    pub(crate) async fn confirm(
        &mut self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        if let ConversationType::UnconfirmedConnection(user_name) = self.conversation_type.clone() {
            let conversation_type = ConversationType::Connection(user_name);
            self.set_conversation_type(executor, notifier, &conversation_type)
                .await?;
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

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct InactiveConversation {
    pub past_members: Vec<UserId>,
}

impl InactiveConversation {
    pub fn new(past_members: Vec<UserId>) -> Self {
        Self { past_members }
    }

    pub fn past_members(&self) -> &[UserId] {
        &self.past_members
    }

    pub fn past_members_mut(&mut self) -> &mut Vec<UserId> {
        &mut self.past_members
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ConversationType {
    // A connection conversation that is not yet confirmed by the other party.
    UnconfirmedConnection(UserId),
    // A connection conversation that is confirmed by the other party and for
    // which we have received the necessary secrets.
    Connection(UserId),
    Group,
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
