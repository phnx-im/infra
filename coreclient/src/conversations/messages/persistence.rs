// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;
use mimi_content::MimiContent;
use phnxtypes::{codec::PhnxCodec, time::TimeStamp};
use rusqlite::{
    named_params, params,
    types::{FromSql, FromSqlError, Type},
    Connection, OptionalExtension, ToSql,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    store::StoreNotifier, utils::persistence::Storable, ContentMessage, ConversationId,
    ConversationMessage, Message,
};

use super::{ErrorMessage, EventMessage};

#[derive(Serialize, Deserialize)]
struct VersionedMessage {
    version: u16,
    // We store the message as bytes, because deserialization depends on
    // other parameters.
    content: Vec<u8>,
}

const CURRENT_MESSAGE_VERSION: u16 = 1;

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

impl VersionedMessage {
    fn to_event_message(&self) -> anyhow::Result<EventMessage> {
        match self.version {
            CURRENT_MESSAGE_VERSION => Ok(PhnxCodec::from_slice::<EventMessage>(&self.content)?),
            _ => bail!("unknown event message version"),
        }
    }

    fn to_mimi_content(&self) -> anyhow::Result<MimiContent> {
        match self.version {
            CURRENT_MESSAGE_VERSION => Ok(PhnxCodec::from_slice::<MimiContent>(&self.content)?),
            _ => bail!("unknown mimi content message version"),
        }
    }

    fn from_event_message(
        event: &EventMessage,
    ) -> Result<VersionedMessage, phnxtypes::codec::Error> {
        Ok(VersionedMessage {
            version: CURRENT_MESSAGE_VERSION,
            content: PhnxCodec::to_vec(&event)?,
        })
    }

    fn from_mimi_content(
        content: &MimiContent,
    ) -> Result<VersionedMessage, phnxtypes::codec::Error> {
        Ok(VersionedMessage {
            version: CURRENT_MESSAGE_VERSION,
            content: PhnxCodec::to_vec(&content)?,
        })
    }

    fn empty_error() -> Self {
        VersionedMessage {
            version: CURRENT_MESSAGE_VERSION,
            content: Vec::new(),
        }
    }
}

use super::{ConversationMessageId, TimestampedMessage};

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
        let sent = row.get(5)?;

        let message;

        match row.get::<_, VersionedMessage>(4) {
            Err(e) => {
                warn!("Versioned message parsing failed: {e}");
                message = Message::Event(EventMessage::Error(ErrorMessage::new(
                    "Versioned message parsing failed".to_owned(),
                )))
            }
            Ok(versioned_message) => match sender_str.as_str() {
                "system" => {
                    message =
                        Message::Event(versioned_message.to_event_message().unwrap_or_else(|e| {
                            warn!("Event parsing failed: {e}");
                            EventMessage::Error(ErrorMessage::new(
                                "Event parsing failed".to_owned(),
                            ))
                        }))
                }
                user_str => {
                    let sender = user_str
                        .strip_prefix("user:")
                        .ok_or(rusqlite::Error::FromSqlConversionFailure(
                            3,
                            Type::Text,
                            Box::new(FromSqlError::InvalidType),
                        ))?
                        .to_string();

                    message = versioned_message
                        .to_mimi_content()
                        .map(|content| {
                            Message::Content(Box::new(ContentMessage {
                                sender,
                                sent,
                                content,
                            }))
                        })
                        .unwrap_or_else(|e| {
                            warn!("Message parsing failed: {e}");
                            Message::Event(EventMessage::Error(ErrorMessage::new(
                                "Message parsing failed".to_owned(),
                            )))
                        });
                }
            },
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
        local_message_id: ConversationMessageId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT
                message_id,
                conversation_id,
                timestamp,
                sender,
                content,
                sent
            FROM conversation_messages
            WHERE message_id = ?",
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
            "SELECT
               message_id,
               conversation_id,
               timestamp,
               sender,
               content,
               sent
            FROM conversation_messages
            WHERE conversation_id = ?
            ORDER BY timestamp DESC
            LIMIT ?",
        )?;
        let mut messages = statement
            .query_map(params![conversation_id, number_of_messages], Self::from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        messages.reverse();
        Ok(messages)
    }

    pub(crate) fn store(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> anyhow::Result<()> {
        let sender = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                format!("user:{}", content_message.sender)
            }
            Message::Event(_) => "system".to_string(),
        };

        let content = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                VersionedMessage::from_mimi_content(&content_message.content)?
            }
            Message::Event(event_message) => VersionedMessage::from_event_message(event_message)?,
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
        notifier.add(self.conversation_message_id);
        notifier.update(self.conversation_id);

        Ok(())
    }

    /// Set the message's sent status in the database and update the message's timestamp.
    pub(super) fn update_sent_status(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        message_id: ConversationMessageId,
        timestamp: TimeStamp,
        sent: bool,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "UPDATE conversation_messages SET timestamp = ?, sent = ? WHERE message_id = ?",
            params![timestamp, sent, message_id],
        )?;
        notifier.update(message_id);
        Ok(())
    }

    /// Get the last content message in the conversation.
    pub(crate) fn last_content_message(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT
                message_id,
                conversation_id,
                timestamp,
                sender,
                content,
                sent
            FROM conversation_messages
            WHERE conversation_id = ? AND sender != 'system'
            ORDER BY timestamp DESC
            LIMIT 1",
        )?;
        statement
            .query_row(params![conversation_id], Self::from_row)
            .optional()
    }

    pub(crate) fn prev_message(
        connection: &Connection,
        message_id: ConversationMessageId,
    ) -> rusqlite::Result<Option<ConversationMessage>> {
        connection
            .query_row(
                "SELECT
                    message_id,
                    conversation_id,
                    timestamp,
                    sender,
                    content,
                    sent
                FROM conversation_messages
                WHERE message_id != :message_id
                    AND timestamp <= (SELECT timestamp FROM conversation_messages WHERE message_id = :message_id)
                ORDER BY timestamp DESC
                LIMIT 1",
                named_params! {
                    ":message_id": message_id.uuid(),
                },
                Self::from_row,
            )
            .optional()
    }

    pub(crate) fn next_message(
        connection: &Connection,
        message_id: ConversationMessageId,
    ) -> rusqlite::Result<Option<ConversationMessage>> {
        connection
            .query_row(
                "SELECT
                    message_id,
                    conversation_id,
                    timestamp,
                    sender,
                    content,
                    sent
                FROM conversation_messages
                WHERE message_id != :message_id
                    AND timestamp >= (SELECT timestamp FROM conversation_messages WHERE message_id = :message_id)
                ORDER BY timestamp ASC
                LIMIT 1",
                named_params! {
                    ":message_id": message_id.uuid(),
                },
                Self::from_row,
            )
            .optional()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::Utc;

    use crate::{
        conversations::persistence::tests::test_conversation, Conversation, EventMessage,
        MimiContent, SystemMessage,
    };

    use super::*;

    pub(crate) fn test_connection() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                &[
                    Conversation::CREATE_TABLE_STATEMENT,
                    ConversationMessage::CREATE_TABLE_STATEMENT,
                ]
                .join("\n"),
            )
            .unwrap();
        connection
    }

    pub(crate) fn test_conversation_message(
        conversation_id: ConversationId,
    ) -> ConversationMessage {
        let conversation_message_id = ConversationMessageId::random();
        let timestamp = Utc::now().into();
        let message = Message::Content(Box::new(ContentMessage {
            sender: "alice@localhost".to_string(),
            sent: false,
            content: MimiContent::simple_markdown_message("Hello world!".to_string()),
        }));
        let timestamped_message = TimestampedMessage { timestamp, message };
        ConversationMessage {
            conversation_message_id,
            conversation_id,
            timestamped_message,
        }
    }

    #[test]
    fn store_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let message = test_conversation_message(conversation.id());

        message.store(&connection, &mut store_notifier)?;
        let loaded = ConversationMessage::load(&connection, message.id())?.unwrap();
        assert_eq!(loaded, message);

        Ok(())
    }

    #[test]
    fn store_load_multiple() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&connection, &mut store_notifier)?;
        message_b.store(&connection, &mut store_notifier)?;

        let loaded = ConversationMessage::load_multiple(&connection, conversation.id(), 2)?;
        assert_eq!(loaded, [message_a, message_b.clone()]);

        let loaded = ConversationMessage::load_multiple(&connection, conversation.id(), 1)?;
        assert_eq!(loaded, [message_b]);

        Ok(())
    }

    #[test]
    fn update_sent_status() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let message = test_conversation_message(conversation.id());
        message.store(&connection, &mut store_notifier)?;

        let loaded = ConversationMessage::load(&connection, message.id())?.unwrap();
        assert!(!loaded.is_sent());

        let sent_at: TimeStamp = Utc::now().into();
        ConversationMessage::update_sent_status(
            &connection,
            &mut store_notifier,
            loaded.id(),
            sent_at,
            true,
        )?;

        let loaded = ConversationMessage::load(&connection, message.id())?.unwrap();
        assert_eq!(&loaded.timestamp(), sent_at.as_ref());
        assert!(loaded.is_sent());

        Ok(())
    }

    #[test]
    fn last_content_message() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&connection, &mut store_notifier)?;
        message_b.store(&connection, &mut store_notifier)?;

        ConversationMessage {
            conversation_id: conversation.id(),
            conversation_message_id: ConversationMessageId::random(),
            timestamped_message: TimestampedMessage {
                timestamp: Utc::now().into(),
                message: Message::Event(EventMessage::System(SystemMessage::Add(
                    "alice@localhost".parse()?,
                    "bob@localhost".parse()?,
                ))),
            },
        }
        .store(&connection, &mut store_notifier)?;

        let loaded = ConversationMessage::last_content_message(&connection, conversation.id())?;
        assert_eq!(loaded, Some(message_b));

        Ok(())
    }

    #[test]
    fn prev_message() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&connection, &mut store_notifier)?;
        message_b.store(&connection, &mut store_notifier)?;

        let loaded = ConversationMessage::prev_message(&connection, message_b.id())?;
        assert_eq!(loaded, Some(message_a));

        Ok(())
    }

    #[test]
    fn next_message() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&connection, &mut store_notifier)?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&connection, &mut store_notifier)?;
        message_b.store(&connection, &mut store_notifier)?;

        let loaded = ConversationMessage::next_message(&connection, message_a.id())?;
        assert_eq!(loaded, Some(message_b));

        Ok(())
    }
}
