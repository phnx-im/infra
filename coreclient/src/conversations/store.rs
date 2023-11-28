// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use openmls::prelude::GroupId;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedGroupId, UserName},
    time::TimeStamp,
};
use rusqlite::Connection;
use tls_codec::DeserializeBytes;
use uuid::Uuid;

use crate::{
    groups::GroupMessage,
    utils::persistence::{DataType, Persistable, PersistableStruct, PersistenceError},
};

use super::{
    messages::ConversationMessage, Conversation, ConversationAttributes, ConversationId,
    ConversationStatus, ConversationType, InactiveConversation,
};

impl Conversation {
    fn create_connection_conversation(
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Result<Self, tls_codec::Error> {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let conversation = Conversation {
            id: ConversationId::try_from(group_id.clone())?,
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_name),
            last_used: TimeStamp::now(),
            attributes,
        };
        Ok(conversation)
    }

    fn create_group_conversation(
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Result<Self, tls_codec::Error> {
        let conversation = Conversation {
            id: ConversationId::try_from(group_id.clone())?,
            group_id: group_id.into(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            last_used: TimeStamp::now(),
            attributes,
        };
        Ok(conversation)
    }

    pub(crate) fn owner_domain(&self) -> Fqdn {
        let qgid = QualifiedGroupId::tls_deserialize_exact(&self.group_id.as_slice()).unwrap();
        qgid.owning_domain
    }

    fn confirm(&mut self) {
        if let ConversationType::UnconfirmedConnection(user_name) = self.conversation_type.clone() {
            self.conversation_type = ConversationType::Connection(user_name);
        }
    }

    fn set_inactive(&mut self, past_members: &[UserName]) {
        self.status = ConversationStatus::Inactive(InactiveConversation {
            past_members: past_members.to_vec(),
        })
    }

    pub fn id(&self) -> ConversationId {
        self.id
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
        conversation_id: &ConversationId,
    ) -> Result<Option<PersistableStruct<Conversation>>, PersistenceError> {
        PersistableStruct::load_one(self.db_connection, Some(conversation_id), None)
    }

    pub(crate) fn get_by_group_id(
        &self,
        group_id: &GroupId,
    ) -> Result<Option<PersistableStruct<Conversation>>, PersistenceError> {
        PersistableStruct::load_one(self.db_connection, None, Some(group_id))
    }

    pub(crate) fn get_all(&self) -> Result<Vec<PersistableStruct<Conversation>>, PersistenceError> {
        PersistableStruct::load_all(self.db_connection)
    }

    pub(crate) fn create_connection_conversation(
        &self,
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Result<PersistableStruct<Conversation>> {
<<<<<<< HEAD
        let payload = Conversation::create_connection_conversation(group_id, user_name, attributes);
=======
        let payload =
            Conversation::create_connection_conversation(group_id, user_name, attributes)?;
>>>>>>> main
        let conversation =
            PersistableStruct::from_connection_and_payload(self.db_connection, payload);
        conversation.persist()?;
        Ok(conversation)
    }

    pub(crate) fn create_group_conversation(
        &self,
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Result<PersistableStruct<Conversation>> {
<<<<<<< HEAD
        let payload = Conversation::create_group_conversation(group_id, attributes);
=======
        let payload = Conversation::create_group_conversation(group_id, attributes)?;
>>>>>>> main
        let conversation =
            PersistableStruct::from_connection_and_payload(self.db_connection, payload);
        conversation.persist()?;
        Ok(conversation)
    }
}

impl Persistable for Conversation {
<<<<<<< HEAD
    type Key = UuidBytes;
    type SecondaryKey = GroupIdBytes;
=======
    type Key = ConversationId;
    type SecondaryKey = GroupId;
>>>>>>> main

    const DATA_TYPE: DataType = DataType::Conversation;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
<<<<<<< HEAD
        &self.group_id
=======
        self.group_id()
>>>>>>> main
    }
}

impl PersistableStruct<'_, Conversation> {
    pub(crate) fn confirm(&mut self) -> Result<(), PersistenceError> {
        self.payload.confirm();
        self.persist()
    }

<<<<<<< HEAD
    pub(crate) fn set_inactive(&mut self, past_members: &[String]) -> Result<(), PersistenceError> {
=======
    pub(crate) fn set_inactive(
        &mut self,
        past_members: &[UserName],
    ) -> Result<(), PersistenceError> {
>>>>>>> main
        self.payload.set_inactive(past_members);
        self.persist()
    }

    pub(crate) fn group_id(&self) -> GroupId {
<<<<<<< HEAD
        self.payload.group_id.as_group_id()
=======
        self.payload.group_id.clone()
>>>>>>> main
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
<<<<<<< HEAD
        conversation_id: &Uuid,
    ) -> Result<Vec<PersistableConversationMessage>, PersistenceError> {
        let uuid_bytes = UuidBytes::from(*conversation_id);
        PersistableConversationMessage::load(self.db_connection, None, Some(&uuid_bytes))
=======
        conversation_id: &ConversationId,
    ) -> Result<Vec<PersistableConversationMessage>, PersistenceError> {
        PersistableConversationMessage::load(self.db_connection, None, Some(&conversation_id))
>>>>>>> main
    }

    pub(crate) fn create(
        &self,
<<<<<<< HEAD
        conversation_id: &Uuid,
=======
        conversation_id: &ConversationId,
>>>>>>> main
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

pub(crate) type PersistableConversationMessage<'a> = PersistableStruct<'a, ConversationMessage>;

impl From<PersistableConversationMessage<'_>> for ConversationMessage {
    fn from(persistable: PersistableConversationMessage) -> Self {
        persistable.payload
    }
}

impl Persistable for ConversationMessage {
    // Message id
<<<<<<< HEAD
    type Key = UuidBytes;

    // Conversation id
    type SecondaryKey = UuidBytes;
=======
    type Key = Uuid;

    // Conversation id
    type SecondaryKey = ConversationId;
>>>>>>> main

    const DATA_TYPE: DataType = DataType::Message;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
<<<<<<< HEAD
        &self.id
=======
        &self.conversation_id
>>>>>>> main
    }
}
