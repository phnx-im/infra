// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::time::TimeStamp;
use rusqlite::{
    params,
    types::{FromSqlError, Type},
    Connection, OptionalExtension,
};
use uuid::Uuid;

use crate::{
    utils::persistence::Storable, ContentMessage, ConversationId, ConversationMessage, Message,
};

use super::TimestampedMessage;

impl Storable for ConversationMessage {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS conversation_messages (
            message_id BLOB PRIMARY KEY,
            conversation_id BLOB NOT NULL,
            timestamp TEXT NOT NULL,
            sender TEXT NOT NULL,
            content BLOB NOT NULL,
            sent BOOLEAN NOT NULL,
            CHECK (sender LIKE 'user:%' OR sender = 'system'),
            FOREIGN KEY (conversation_id) REFERENCES conversations(conversation_id) DEFERRABLE INITIALLY DEFERRED
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let conversation_message_id = row.get(0)?;
        let conversation_id = row.get(1)?;
        let timestamp = row.get(2)?;
        let sender_str: String = row.get(3)?;
        let message: Vec<u8> = row.get(4)?;
        let sent = row.get(5)?;

        let message = if sender_str == "system" {
            let event_message = serde_json::from_slice(&message).map_err(|e| {
                log::error!("Failed to deserialize content message: {}", e);
                rusqlite::Error::FromSqlConversionFailure(4, Type::Blob, Box::new(e))
            })?;
            Message::Event(event_message)
        } else {
            let content = serde_json::from_slice(&message).map_err(|e| {
                log::error!("Failed to deserialize content message: {}", e);
                rusqlite::Error::FromSqlConversionFailure(4, Type::Blob, Box::new(e))
            })?;
            let sender = sender_str
                .strip_prefix("user:")
                .ok_or(rusqlite::Error::FromSqlConversionFailure(
                    3,
                    Type::Text,
                    Box::new(FromSqlError::InvalidType),
                ))?
                .to_string();
            let content_message = ContentMessage {
                sender,
                sent,
                content,
            };
            Message::Content(content_message)
        };
        let timestamped_message = TimestampedMessage { timestamp, message };

        Ok(ConversationMessage {
            conversation_message_id,
            conversation_id,
            timestamped_message,
        })
    }
}

impl ConversationMessage {
    pub(crate) fn load(
        connection: &Connection,
        local_message_id: &Uuid,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT message_id, conversation_id, timestamp, sender, content, sent FROM conversation_messages WHERE message_id = ?",
        )?;
        statement
            .query_row(params![local_message_id], Self::from_row)
            .optional()
    }

    pub(crate) fn load_multiple(
        connection: &Connection,
        conversation_id: ConversationId,
        number_of_messages: u32,
    ) -> Result<Vec<ConversationMessage>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT * FROM (SELECT message_id, conversation_id, timestamp, sender, content, sent FROM conversation_messages WHERE conversation_id = ? ORDER BY timestamp DESC LIMIT ?) ORDER BY timestamp ASC",
        )?;
        let messages = statement
            .query_map(params![conversation_id, number_of_messages], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    pub(crate) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        let sender = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                format!("user:{}", content_message.sender)
            }
            Message::Event(_) => "system".to_string(),
        };
        let content = match &self.timestamped_message.message {
            Message::Content(content_message) => serde_json::to_vec(content_message.content())
                .map_err(|e| {
                    log::error!("Failed to serialize MIMI content: {}", e);
                    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
                })?,
            Message::Event(event_message) => serde_json::to_vec(event_message).map_err(|e| {
                log::error!("Failed to serialize event message: {}", e);
                rusqlite::Error::ToSqlConversionFailure(Box::new(e))
            })?,
        };
        connection.execute(
            "INSERT INTO conversation_messages (message_id, conversation_id, timestamp, sender, content, sent) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                self.conversation_message_id,
                self.conversation_id,
                self.timestamped_message.timestamp,
                sender,
                content,
                match &self.timestamped_message.message {
                    Message::Content(content_message) => content_message.sent,
                    Message::Event(_) => true,
                },
            ],
        )?;
        Ok(())
    }

    /// Set the message's sent status in the database and update the message's timestamp.
    pub(super) fn update_sent_status(
        &self,
        connection: &Connection,
        timestamp: TimeStamp,
        sent: bool,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "UPDATE conversation_messages SET timestamp = ?, sent = ? WHERE message_id = ?",
            params![timestamp, sent, self.conversation_message_id],
        )?;
        Ok(())
    }

    /// Get the last content message in the conversation.
    pub(crate) fn last_content_message(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT message_id, conversation_id, timestamp, sender, content, sent FROM conversation_messages WHERE conversation_id = ? AND sender != 'system' ORDER BY timestamp DESC LIMIT 1",
        )?;
        statement
            .query_row(params![conversation_id], Self::from_row)
            .optional()
    }
}
