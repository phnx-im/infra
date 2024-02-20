// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use openmls::prelude::GroupId;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedGroupId, UserName},
    time::TimeStamp,
};
use rusqlite::{named_params, Connection};
use tls_codec::{DeserializeBytes, Serialize};
use uuid::Uuid;

use crate::{
    groups::GroupMessage,
    utils::persistence::{
        DataType, Persistable, PersistableStruct, PersistenceError, SqlFieldDefinition, SqlKey,
    },
};

use super::{
    messages::ConversationMessage, Conversation, ConversationAttributes, ConversationId,
    ConversationPayload, ConversationStatus, ConversationType, InactiveConversation,
};

impl Conversation {
    fn create_connection_conversation(
        group_id: GroupId,
        user_name: UserName,
        attributes: ConversationAttributes,
    ) -> Result<Self, tls_codec::Error> {
        // To keep things simple and to make sure that conversation ids are the
        // same across users, we derive the conversation id from the group id.
        let conversation_payload = ConversationPayload {
            status: ConversationStatus::Active,
            conversation_type: ConversationType::UnconfirmedConnection(user_name),
            attributes,
        };
        let conversation = Conversation {
            id: ConversationId::try_from(group_id.clone())?,
            group_id: group_id.into(),
            conversation_payload,
            last_used: TimeStamp::now(),
            last_read: TimeStamp::now(),
        };
        Ok(conversation)
    }

    fn create_group_conversation(
        group_id: GroupId,
        attributes: ConversationAttributes,
    ) -> Result<Self, tls_codec::Error> {
        let conversation_payload = ConversationPayload {
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            attributes,
        };
        let conversation = Conversation {
            id: ConversationId::try_from(group_id.clone())?,
            group_id: group_id.into(),
            conversation_payload,
            last_used: TimeStamp::now(),
            last_read: TimeStamp::now(),
        };
        Ok(conversation)
    }

    pub(crate) fn owner_domain(&self) -> Fqdn {
        let qgid =
            QualifiedGroupId::tls_deserialize_exact_bytes(&self.group_id.as_slice()).unwrap();
        qgid.owning_domain
    }

    fn confirm(&mut self) {
        if let ConversationType::UnconfirmedConnection(user_name) =
            self.conversation_payload.conversation_type.clone()
        {
            self.conversation_payload.conversation_type = ConversationType::Connection(user_name);
        }
    }

    fn set_inactive(&mut self, past_members: &[UserName]) {
        self.conversation_payload.status = ConversationStatus::Inactive(InactiveConversation {
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

    /// Count the number of unread messages in the conversation and return the
    /// result.
    pub(crate) fn unread_message_count(
        &self,
        conversation_id: ConversationId,
    ) -> Result<u32, PersistenceError> {
        let conversation_message_store = ConversationMessageStore::from(self.db_connection);
        conversation_message_store.unread_message_count(conversation_id)
    }

    /// Set the `last_read` marker of all conversations with the given
    /// [`ConversationId`]s to the given timestamps. This is used to mark all
    /// messages up to this timestamp as read.
    pub(crate) fn mark_as_read<
        'b,
        T: 'b + IntoIterator<Item = (&'b ConversationId, &'b TimeStamp)>,
    >(
        &self,
        mark_as_read_data: T,
    ) -> Result<(), PersistenceError> {
        // TOOD: This should be a transaction
        let transaction = self.db_connection;
        for (conversation_id, timestamp) in mark_as_read_data.into_iter() {
            let statement_str = format!(
                "UPDATE {} SET last_read = :timestamp WHERE primary_key = :conversation_id",
                DataType::Conversation.to_sql_key()
            );
            let mut stmt = transaction.prepare(&statement_str)?;
            stmt.execute(named_params! {
                ":timestamp": timestamp.time(),
                ":conversation_id": conversation_id.to_sql_key(),
            })?;
        }
        //transaction.commit()?;
        Ok(())
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

    fn additional_fields() -> Vec<SqlFieldDefinition> {
        vec![
            ("last_used", "TEXT").into(),
            ("last_read", "TEXT").into(),
            ("payload", "BLOB").into(),
        ]
    }

    fn get_sql_values(&self) -> Result<Vec<Box<dyn rusqlite::ToSql>>, PersistenceError> {
        let conversation_payload = serde_json::to_vec(&self.conversation_payload)?;
        Ok(vec![
            Box::new(self.last_used.time()),
            Box::new(self.last_read.time()),
            Box::new(conversation_payload),
        ])
    }

    fn try_from_row(row: &rusqlite::Row) -> Result<Self, PersistenceError> {
        let id_text = row.get::<_, String>(1)?;
        let id = Uuid::from_str(&id_text)
            .map_err(|e| {
                anyhow!(
                "Error converting message UUID from string to Uuid: {} when reading from SQLite",
                e
            )
            })?
            .into();

        let qgid_text = row.get::<_, String>(2)?;
        let qualified_group_id = QualifiedGroupId::try_from(qgid_text).map_err(|e| {
            anyhow!(
                "Invalid string representation of qualified group id: {} when reading from SQLite",
                e
            )
        })?;
        let group_id =
            GroupId::from_slice(&qualified_group_id.tls_serialize_detached().map_err(|e| {
                PersistenceError::ConversionError(anyhow!(
                    "Failed to serialize qualified group id : {:?}",
                    e
                ))
            })?);

        let last_used_date_time: DateTime<Utc> = row.get(3)?;
        let last_used = last_used_date_time.into();
        let last_read_date_time: DateTime<Utc> = row.get(4)?;
        let last_read = last_read_date_time.into();

        let conversation_payload_bytes: Vec<u8> = row.get(5)?;
        let conversation_payload = serde_json::from_slice(&conversation_payload_bytes)?;
        Ok(Conversation {
            id,
            group_id,
            last_used,
            last_read,
            conversation_payload,
        })
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
        self.payload
            .conversation_payload
            .attributes
            .conversation_picture_option = conversation_picture;
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

    fn unread_message_count(
        &self,
        conversation_id: ConversationId,
    ) -> Result<u32, PersistenceError> {
        let messages_table_name = DataType::Message.to_sql_key();
        let conversations_table_name = DataType::Conversation.to_sql_key();
        let statement_str = format!(
            "SELECT COUNT(*) FROM {messages_table_name} WHERE secondary_key = :conversation_id AND timestamp > (SELECT last_read FROM {conversations_table_name} WHERE primary_key = :conversation_id)",
        );
        let mut stmt = self.db_connection.prepare(&statement_str)?;
        let count: u32 = stmt.query_row(
            named_params! {":conversation_id": conversation_id.to_sql_key()},
            |row| row.get(0),
        )?;
        Ok(count)
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
        vec![("timestamp", "TEXT").into(), ("message", "BLOB").into()]
    }

    fn get_sql_values(&self) -> Result<Vec<Box<dyn rusqlite::ToSql>>, PersistenceError> {
        let message_bytes = serde_json::to_vec(&self.message)?;
        Ok(vec![
            Box::new(self.timestamp.time()),
            Box::new(message_bytes),
        ])
    }

    fn try_from_row(row: &rusqlite::Row) -> Result<Self, PersistenceError> {
        let id_text = row.get::<_, String>(1)?;
        let id = Uuid::from_str(&id_text).map_err(|e| {
            anyhow!(
                "Error converting message UUID from string to Uuid: {} when reading from SQLite",
                e
            )
        })?;
        let conversation_uuid_text = row.get::<_, String>(2)?;
        let conversation_id: ConversationId = Uuid::from_str(&conversation_uuid_text)
            .map_err(|e| {
                anyhow!(
            "Error converting conversation UUID from string to Uuid: {} when reading from SQLite",
            e
        )
            })?
            .into();
        let date_time: DateTime<Utc> = row.get(3)?;
        let timestamp = date_time.into();

        let message_bytes: Vec<u8> = row.get(4)?;
        let message = serde_json::from_slice(&message_bytes)?;
        Ok(ConversationMessage {
            conversation_id,
            id,
            timestamp,
            message,
        })
    }
}
