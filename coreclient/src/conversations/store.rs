// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use openmls::prelude::GroupId;
use phnxtypes::identifiers::{Fqdn, QualifiedGroupId, UserName};
use rusqlite::Connection;
use tls_codec::DeserializeBytes;
use uuid::Uuid;

use crate::{
    groups::GroupMessage,
    types::*,
    utils::{
        persistence::{DataType, Persistable, PersistenceError},
        Timestamp,
    },
};

impl Conversation {
    fn create_connection_conversation(
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Self {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let uuid_bytes = UuidBytes::from_group_id(&group_id);
        Conversation {
            id: uuid_bytes.clone(),
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_name.to_string()),
            last_used: Timestamp::now().as_u64(),
            attributes,
        }
    }

    fn create_group_conversation(group_id: GroupId, attributes: ConversationAttributes) -> Self {
        let uuid_bytes = UuidBytes::from_group_id(&group_id);
        Conversation {
            id: uuid_bytes.clone(),
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            last_used: Timestamp::now().as_u64(),
            attributes,
        }
    }

    pub(crate) fn owner_domain(&self) -> Fqdn {
        let qgid = QualifiedGroupId::tls_deserialize_exact(&self.group_id.bytes).unwrap();
        qgid.owning_domain
    }

    fn confirm(&mut self) {
        if let ConversationType::UnconfirmedConnection(user_name) = self.conversation_type.clone() {
            self.conversation_type = ConversationType::Connection(user_name);
        }
    }

    fn set_inactive(&mut self, past_members: &[String]) {
        self.status = ConversationStatus::Inactive(InactiveConversation {
            past_members: past_members.iter().map(|m| m.to_owned()).collect(),
        })
    }

    pub(crate) fn id(&self) -> Uuid {
        self.id.as_uuid()
    }
}

pub(crate) struct ConversationStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for ConversationStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> ConversationStore<'a> {
    pub(crate) fn get_by_conversation_id(
        &self,
        conversation_id: &Uuid,
    ) -> Result<Option<PersistableConversation>, PersistenceError> {
        let uuid_bytes = UuidBytes::from(*conversation_id);
        PersistableConversation::load_one(self.db_connection, Some(&uuid_bytes), None)
    }

    pub(crate) fn get_by_group_id(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<PersistableConversation>, PersistenceError> {
        let group_id_bytes = GroupIdBytes::from(group_id.clone());
        PersistableConversation::load_one(self.db_connection, None, Some(&group_id_bytes))
    }

    pub(crate) fn get_all(&self) -> Result<Vec<PersistableConversation>, PersistenceError> {
        PersistableConversation::load_all(self.db_connection)
    }

    pub(crate) fn create_connection_conversation(
        &self,
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Result<PersistableConversation> {
        let payload = Conversation::create_connection_conversation(group_id, user_name, attributes);
        let conversation =
            PersistableConversation::from_connection_and_payload(self.db_connection, payload);
        conversation.persist()?;
        Ok(conversation)
    }

    pub(crate) fn create_group_conversation(
        &self,
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Result<PersistableConversation> {
        let payload = Conversation::create_group_conversation(group_id, attributes);
        let conversation =
            PersistableConversation::from_connection_and_payload(self.db_connection, payload);
        conversation.persist()?;
        Ok(conversation)
    }
}

pub(crate) struct PersistableConversation<'a> {
    connection: &'a Connection,
    payload: Conversation,
}

impl std::ops::Deref for PersistableConversation<'_> {
    type Target = Conversation;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<'a> Persistable<'a> for PersistableConversation<'a> {
    type Key = UuidBytes;
    type SecondaryKey = GroupIdBytes;

    const DATA_TYPE: DataType = DataType::Conversation;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.group_id
    }

    type Payload = Conversation;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}

impl PersistableConversation<'_> {
    pub(crate) fn confirm(&mut self) -> Result<(), PersistenceError> {
        self.payload.confirm();
        self.persist()
    }

    pub(crate) fn set_inactive(&mut self, past_members: &[String]) -> Result<(), PersistenceError> {
        self.payload.set_inactive(past_members);
        self.persist()
    }

    pub(crate) fn group_id(&self) -> GroupId {
        self.payload.group_id.as_group_id()
    }

    pub(crate) fn convert_for_export(self) -> Conversation {
        self.payload
    }
}

pub(crate) struct ConversationMessageStore<'a> {
    db_connection: &'a Connection,
}

impl<'a> From<&'a Connection> for ConversationMessageStore<'a> {
    fn from(db_connection: &'a Connection) -> Self {
        Self { db_connection }
    }
}

impl<'a> ConversationMessageStore<'a> {
    pub(crate) fn get_by_conversation_id(
        &self,
        conversation_id: &Uuid,
    ) -> Result<Vec<PersistableConversationMessage>, PersistenceError> {
        let uuid_bytes = UuidBytes::from(*conversation_id);
        PersistableConversationMessage::load(self.db_connection, None, Some(&uuid_bytes))
    }

    pub(crate) fn create(
        &self,
        conversation_id: &Uuid,
        group_message: GroupMessage,
    ) -> Result<PersistableConversationMessage, PersistenceError> {
        let payload = ConversationMessage::new(conversation_id.clone(), group_message);
        let conversation_message = PersistableConversationMessage::from_connection_and_payload(
            self.db_connection,
            payload,
        );
        conversation_message.persist()?;
        Ok(conversation_message)
    }
}

pub(crate) struct PersistableConversationMessage<'a> {
    connection: &'a Connection,
    payload: ConversationMessage,
}

impl From<PersistableConversationMessage<'_>> for ConversationMessage {
    fn from(persistable: PersistableConversationMessage) -> Self {
        persistable.payload
    }
}

impl std::ops::Deref for PersistableConversationMessage<'_> {
    type Target = ConversationMessage;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<'a> Persistable<'a> for PersistableConversationMessage<'a> {
    // Message id
    type Key = UuidBytes;

    // Conversation id
    type SecondaryKey = UuidBytes;

    const DATA_TYPE: DataType = DataType::Message;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.id
    }

    type Payload = ConversationMessage;

    fn connection(&self) -> &Connection {
        self.connection
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn from_connection_and_payload(conn: &'a Connection, payload: Self::Payload) -> Self {
        Self {
            connection: conn,
            payload,
        }
    }
}
