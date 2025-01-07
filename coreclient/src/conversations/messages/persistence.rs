// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{codec::PhnxCodec, time::TimeStamp};
use rusqlite::{
    named_params, params,
    types::{FromSql, FromSqlError, Type},
    Connection, OptionalExtension, ToSql,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    store::StoreNotifier, utils::persistence::Storable, ContentMessage, ConversationId,
    ConversationMessage, Message,
};

// When adding a variant to this enum, the new variant must be called
// `CurrentVersion` and the current version must be renamed to `VX`, where `X`
// is the next version number. The content type of the old `CurrentVersion` must
// be renamed and otherwise preserved to ensure backwards compatibility.
#[derive(Serialize, Deserialize)]
enum VersionedMessage {
    // We store the message as bytes, because deserialization depends on
    // other parameters.
    CurrentVersion(Vec<u8>),
}

impl FromSql for VersionedMessage {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        let versioned_message = PhnxCodec::from_slice(bytes)?;
        Ok(versioned_message)
    }
}

impl ToSql for VersionedMessage {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let bytes = PhnxCodec::to_vec(self)?;
        Ok(rusqlite::types::ToSqlOutput::from(bytes))
    }
}

enum MessageInputs {
    System,
    User(String, bool), // sender, sent
}

enum VersionedMessageInputs {
    CurrentVersion(Vec<u8>, MessageInputs),
}

impl Message {
    // For future message types, the additional inputs to this function might
    // have to be adjusted.
    fn from_versioned_message(
        versioned_message: VersionedMessageInputs,
    ) -> Result<Self, phnxtypes::codec::Error> {
        match versioned_message {
            VersionedMessageInputs::CurrentVersion(message_bytes, inputs) => match inputs {
                MessageInputs::System => {
                    let event_message = PhnxCodec::from_slice(&message_bytes)?;
                    Ok(Message::Event(event_message))
                }
                MessageInputs::User(sender, sent) => {
                    let content = PhnxCodec::from_slice(&message_bytes)?;
                    let content_message = ContentMessage {
                        sender,
                        sent,
                        content,
                    };
                    Ok(Message::Content(Box::new(content_message)))
                }
            },
        }
    }

    fn to_versioned_message(&self) -> Result<VersionedMessage, phnxtypes::codec::Error> {
        let message_bytes = match self {
            Message::Event(event_message) => PhnxCodec::to_vec(event_message)?,
            Message::Content(content_message) => PhnxCodec::to_vec(content_message.content())?,
        };
        Ok(VersionedMessage::CurrentVersion(message_bytes))
    }
}

use super::{ConversationMessageId, ConversationMessageNeighbor, TimestampedMessage};

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
        let versioned_message: VersionedMessage = row.get(4)?;
        let sent = row.get(5)?;
        let is_read = row.get(6)?;

        let versioned_message_inputs = match versioned_message {
            VersionedMessage::CurrentVersion(bytes) => {
                let inputs = match sender_str.as_str() {
                    "system" => MessageInputs::System,
                    user_str => {
                        let sender = user_str
                            .strip_prefix("user:")
                            .ok_or(rusqlite::Error::FromSqlConversionFailure(
                                3,
                                Type::Text,
                                Box::new(FromSqlError::InvalidType),
                            ))?
                            .to_string();
                        MessageInputs::User(sender, sent)
                    }
                };
                VersionedMessageInputs::CurrentVersion(bytes, inputs)
            }
        };
        let message = Message::from_versioned_message(versioned_message_inputs).map_err(|e| {
            log::error!("Failed to deserialize content message: {}", e);
            rusqlite::Error::FromSqlConversionFailure(4, Type::Blob, Box::new(e))
        })?;

        let timestamped_message = TimestampedMessage {
            timestamp,
            message,
            is_read,
        };

        Ok(ConversationMessage {
            conversation_message_id,
            conversation_id,
            timestamped_message,
            neighbors: Default::default(),
        })
    }
}

impl ConversationMessageNeighbor {
    fn from_row(
        row: &rusqlite::Row,
        message_id_idx: usize,
        sender_idx: usize,
        timestamp_idx: usize,
    ) -> rusqlite::Result<Option<Self>> {
        let message_id = row.get(message_id_idx)?;
        let sender = row.get(sender_idx)?;
        let timestamp: Option<String> = row.get(timestamp_idx)?;
        let timestamp = timestamp.and_then(|s| s.parse().ok());
        match (message_id, sender, timestamp) {
            (Some(message_id), Some(sender), Some(timestamp)) => Ok(Some(Self {
                message_id,
                sender,
                timestamp,
            })),
            _ => Ok(None),
        }
    }
}

impl ConversationMessage {
    pub(crate) fn load(
        connection: &Connection,
        local_message_id: &Uuid,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT cm.message_id, cm.conversation_id, cm.timestamp, cm.sender, cm.content, cm.sent, cm.timestamp <= c.last_read AS is_read
            FROM conversation_messages cm
            INNER JOIN conversations c ON c.conversation_id = cm.conversation_id
            WHERE message_id = ?"
        )?;
        statement
            .query_row(params![local_message_id], Self::from_row)
            .optional()
    }

    pub(crate) fn load_with_neighbors(
        connection: &Connection,
        local_message_id: &Uuid,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT
                cm.message_id,
                cm.conversation_id,
                cm.timestamp,
                cm.sender,
                cm.content,
                cm.sent,
                cm.timestamp <= c.last_read AS is_read,
                prev.message_id,
                prev.sender,
                prev.timestamp,
                next.message_id,
                next.sender,
                next.timestamp
            FROM conversation_messages cm
            LEFT JOIN
                conversation_messages prev
                ON prev.message_id = (
                    SELECT prev_inner.message_id
                    FROM conversation_messages prev_inner
                    WHERE
                        prev_inner.conversation_id = cm.conversation_id
                        AND prev_inner.timestamp <= cm.timestamp
                        AND prev_inner.message_id != cm.message_id
                    ORDER BY prev_inner.timestamp desc
                    LIMIT 1
                )
            LEFT JOIN
                conversation_messages next
                ON next.message_id = (
                    SELECT next_inner.message_id
                    FROM conversation_messages next_inner
                    WHERE
                        next_inner.conversation_id = cm.conversation_id
                        AND next_inner.timestamp >= cm.timestamp
                        AND next_inner.message_id != cm.message_id
                    ORDER BY next_inner.timestamp asc
                    LIMIT 1
                )
            INNER JOIN conversations c ON c.conversation_id = cm.conversation_id
            WHERE cm.message_id = ?",
        )?;
        statement
            .query_row(params![local_message_id], |row| {
                let mut message = Self::from_row(row)?;
                message.neighbors.prev = ConversationMessageNeighbor::from_row(row, 7, 8, 9)?;
                message.neighbors.next = ConversationMessageNeighbor::from_row(row, 10, 11, 12)?;
                Ok(message)
            })
            .optional()
    }

    pub(crate) fn load_multiple(
        connection: &Connection,
        conversation_id: ConversationId,
        number_of_messages: u32,
    ) -> Result<Vec<ConversationMessage>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT *
            FROM (
                SELECT
                   cm.message_id,
                   cm.conversation_id,
                   cm.timestamp,
                   cm.sender,
                   cm.content,
                   cm.sent,
                   cm.timestamp <= c.last_read AS is_read
                FROM conversation_messages cm
                INNER JOIN conversations c ON c.conversation_id = cm.conversation_id
                WHERE cm.conversation_id = ?
                ORDER BY timestamp DESC
                LIMIT ?
            ) AS messages
            ORDER BY timestamp ASC;",
        )?;
        let messages = statement
            .query_map(params![conversation_id, number_of_messages], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    pub(crate) fn store(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        let sender = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                format!("user:{}", content_message.sender)
            }
            Message::Event(_) => "system".to_string(),
        };
        let content = self.timestamped_message.message.to_versioned_message()?;
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
        notifier.add(self.conversation_message_id);
        Ok(())
    }

    /// Set the message's sent status in the database and update the message's timestamp.
    pub(super) fn update_sent_status(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
        timestamp: TimeStamp,
        sent: bool,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "UPDATE conversation_messages SET timestamp = ?, sent = ? WHERE message_id = ?",
            params![timestamp, sent, self.conversation_message_id],
        )?;
        notifier.update(self.conversation_message_id);
        Ok(())
    }

    /// Get the last content message in the conversation.
    pub(crate) fn last_content_message(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT message_id, conversation_id, timestamp, sender, content, sent
            FROM conversation_messages
            WHERE conversation_id = ? AND sender != 'system'
            ORDER BY timestamp DESC LIMIT 1",
        )?;
        statement
            .query_row(params![conversation_id], Self::from_row)
            .optional()
    }

    pub(crate) fn id_from_rev_offset(
        connection: &Connection,
        conversation_id: ConversationId,
        offset: usize,
    ) -> rusqlite::Result<Option<ConversationMessageId>> {
        // TODO: Add an index on timestamp, otherwise this query is slow
        connection
            .query_row(
                "SELECT message_id FROM conversation_messages
                WHERE conversation_id = :conversation_id
                ORDER BY timestamp DESC
                LIMIT 1 OFFSET :offset",
                named_params! {
                    ":conversation_id": conversation_id,
                    ":offset": offset,
                },
                |row| row.get(0),
            )
            .optional()
    }

    pub(crate) fn rev_offset_from_id(
        connection: &Connection,
        conversation_id: ConversationId,
        message_id: ConversationMessageId,
    ) -> rusqlite::Result<Option<usize>> {
        let count: usize = connection.query_row(
            "SELECT COUNT(message_id) FROM conversation_messages
            WHERE conversation_id = :conversation_id
            AND timestamp >= (
                SELECT timestamp FROM conversation_messages
                WHERE message_id = :message_id
            )",
            named_params! {
                ":conversation_id": conversation_id,
                ":message_id": message_id,
            },
            |row| row.get(0),
        )?;
        match count {
            0 => Ok(None),
            n => Ok(Some(n - 1)),
        }
    }
}
