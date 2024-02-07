// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use openmls::prelude::GroupId;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedGroupId, UserName},
    time::TimeStamp,
};
use rusqlite::{named_params, Connection};
use tls_codec::DeserializeBytes;
use uuid::Uuid;

use crate::{
    groups::GroupMessage,
    utils::persistence::{
        DataType, Persistable, PersistableStruct, PersistenceError, SqlFieldDefinition, SqlKey,
    },
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
        let qgid =
            QualifiedGroupId::tls_deserialize_exact_bytes(&self.group_id.as_slice()).unwrap();
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
        let payload =
            Conversation::create_connection_conversation(group_id, user_name, attributes)?;
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
        let payload = Conversation::create_group_conversation(group_id, attributes)?;
        let conversation =
            PersistableStruct::from_connection_and_payload(self.db_connection, payload);
        conversation.persist()?;
        Ok(conversation)
    }
}

impl Persistable for Conversation {
    type Key = ConversationId;
    type SecondaryKey = GroupId;

    const DATA_TYPE: DataType = DataType::Conversation;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        self.group_id()
    }
}

impl PersistableStruct<'_, Conversation> {
    pub(crate) fn confirm(&mut self) -> Result<(), PersistenceError> {
        self.payload.confirm();
        self.persist()
    }

    pub(crate) fn set_inactive(
        &mut self,
        past_members: &[UserName],
    ) -> Result<(), PersistenceError> {
        self.payload.set_inactive(past_members);
        self.persist()
    }

    pub(crate) fn group_id(&self) -> GroupId {
        self.payload.group_id.clone()
    }

    pub(crate) fn convert_for_export(self) -> Conversation {
        self.payload
    }

    pub(crate) fn set_conversation_picture(
        &mut self,
        conversation_picture: Option<Vec<u8>>,
    ) -> Result<(), PersistenceError> {
        self.payload.attributes.conversation_picture_option = conversation_picture;
        self.persist()
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
        conversation_id: &ConversationId,
    ) -> Result<Vec<PersistableConversationMessage>, PersistenceError> {
        PersistableConversationMessage::load(self.db_connection, None, Some(&conversation_id))
    }

    pub(crate) fn create(
        &self,
        conversation_id: &ConversationId,
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

    /// Mark all messages in the conversation with the given conversation id and
    /// with a timestamp older than the given timestamp as read.
    pub(crate) fn mark_as_read(
        &self,
        conversation_id: ConversationId,
        timestamp: TimeStamp,
    ) -> Result<(), PersistenceError> {
        let data_type = DataType::Message;
        let data_type_sql_key = data_type.to_sql_key();
        let statement_str = format!(
            "UPDATE {data_type_sql_key} SET read = true FROM {data_type_sql_key} WHERE secondary_key = :secondary_key AND timestamp < :timestamp",
        );
        let mut stmt = match self.db_connection.prepare(&statement_str) {
            Ok(stmt) => stmt,
            // If the table does not exist, we create it and try again.
            Err(e) => match e {
                rusqlite::Error::SqliteFailure(_, Some(ref error_string)) => {
                    let expected_error_string = format!("no such table: {}", data_type_sql_key);
                    // If there is no table, there are no messages to be marked.
                    if error_string == &expected_error_string {
                        return Ok(());
                    } else {
                        return Err(e.into());
                    }
                }
                _ => return Err(e.into()),
            },
        };
        stmt.insert(
            named_params! {":secondary_key": conversation_id.to_sql_key(),":timestamp": timestamp.as_i64()},
        )?;

        Ok(())
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
    type Key = Uuid;

    // Conversation id
    type SecondaryKey = ConversationId;

    const DATA_TYPE: DataType = DataType::Message;

    fn key(&self) -> &Self::Key {
        &self.id
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &self.conversation_id
    }

    fn additional_fields() -> Vec<SqlFieldDefinition> {
        vec![
            ("timestamp", "i64 DEFAULT CURRENT_TIMESTAMP").into(),
            ("message", "BLOB").into(),
            ("read", "BOOLEAN DEFAULT false").into(),
        ]
    }

    fn get_sql_values(&self) -> Result<Vec<Box<dyn rusqlite::ToSql>>, PersistenceError> {
        let message_bytes = serde_json::to_vec(&self.message)?;
        Ok(vec![
            Box::new(self.timestamp.as_i64()),
            Box::new(message_bytes),
            Box::new(self.read),
        ])
    }

    fn try_from_row(row: &rusqlite::Row) -> Result<Self, PersistenceError> {
        let conversation_uuid: Uuid = row.get(1)?;
        let conversation_id = ConversationId::from(conversation_uuid);
        let id = row.get(2)?;
        let timestamp_u64: u64 = row.get(3)?;
        let timestamp = timestamp_u64.try_into().map_err(|e| {
            anyhow!(
                "Error converting timestamp from u64 to TimeStamp: {} when reading from SQLite",
                e
            )
        })?;

        let message_bytes: Vec<u8> = row.get(4)?;
        let message = serde_json::from_slice(&message_bytes)?;
        let read = row.get(5)?;
        Ok(ConversationMessage {
            conversation_id,
            id,
            timestamp,
            message,
            read,
        })
    }
}
