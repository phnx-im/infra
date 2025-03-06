// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    codec::{self, PhnxCodec},
    time::TimeStamp,
};
use rusqlite::{
    types::{FromSql, FromSqlError, Type, ValueRef},
    ToSql,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    encode::IsNull, error::BoxDynError, query, query_as, Database, Decode, Encode, Sqlite,
    SqliteExecutor,
};
use tokio_stream::StreamExt;
use tracing::error;

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

impl sqlx::Type<Sqlite> for VersionedMessage {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&[u8] as sqlx::Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for VersionedMessage {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        let bytes = PhnxCodec::to_vec(self)?;
        <Vec<u8> as Encode<Sqlite>>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for VersionedMessage {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as Decode<Sqlite>>::decode(value)?;
        let versioned_message = PhnxCodec::from_slice(bytes)?;
        Ok(versioned_message)
    }
}

impl FromSql for VersionedMessage {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
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
        let versioned_message: VersionedMessage = row.get(4)?;
        let sent = row.get(5)?;

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
        let message =
            Message::from_versioned_message(versioned_message_inputs).map_err(|error| {
                error!(%error, "Failed to deserialize content message");
                rusqlite::Error::FromSqlConversionFailure(4, Type::Blob, Box::new(error))
            })?;

        let timestamped_message = TimestampedMessage { timestamp, message };

        Ok(ConversationMessage {
            conversation_message_id,
            conversation_id,
            timestamped_message,
        })
    }
}

struct SqlConversationMessage {
    message_id: ConversationMessageId,
    conversation_id: ConversationId,
    timestamp: TimeStamp,
    sender: String,
    content: VersionedMessage,
    sent: bool,
}

#[derive(thiserror::Error, Debug)]
enum VersionedMessageError {
    #[error("Invalid user prefix")]
    InvalidUserPrefix,
    #[error(transparent)]
    Codec(#[from] codec::Error),
}

impl From<VersionedMessageError> for sqlx::Error {
    fn from(value: VersionedMessageError) -> Self {
        sqlx::Error::Decode(Box::new(value))
    }
}

impl TryFrom<SqlConversationMessage> for ConversationMessage {
    type Error = VersionedMessageError;

    fn try_from(
        SqlConversationMessage {
            message_id,
            conversation_id,
            timestamp,
            sender,
            content,
            sent,
        }: SqlConversationMessage,
    ) -> Result<Self, Self::Error> {
        let versioned_message_inputs = match content {
            VersionedMessage::CurrentVersion(bytes) => {
                let inputs = match sender.as_str() {
                    "system" => MessageInputs::System,
                    user_str => {
                        let sender = user_str
                            .strip_prefix("user:")
                            .ok_or(VersionedMessageError::InvalidUserPrefix)?
                            .to_string();
                        MessageInputs::User(sender, sent)
                    }
                };
                VersionedMessageInputs::CurrentVersion(bytes, inputs)
            }
        };
        let message = Message::from_versioned_message(versioned_message_inputs)?;
        let timestamped_message = TimestampedMessage { timestamp, message };
        Ok(ConversationMessage {
            conversation_message_id: message_id,
            conversation_id,
            timestamped_message,
        })
    }
}

impl ConversationMessage {
    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        message_id: ConversationMessageId,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender,
                content As "content: _",
                sent
            FROM conversation_messages WHERE message_id = ?"#,
            message_id,
        )
        .fetch_optional(executor)
        .await
        .map(|record| {
            record
                .map(TryFrom::try_from)
                .transpose()
                .map_err(From::from)
        })?
    }

    pub(crate) async fn load_multiple(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ConversationId,
        number_of_messages: u32,
    ) -> sqlx::Result<Vec<ConversationMessage>> {
        let messages: sqlx::Result<Vec<ConversationMessage>> = query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender,
                content AS "content: _",
                sent
            FROM conversation_messages
            WHERE conversation_id = ?
            ORDER BY timestamp DESC
            LIMIT ?"#,
            conversation_id,
            number_of_messages,
        )
        .fetch(executor)
        .map(|res| res?.try_into().map_err(From::from))
        .collect()
        .await;
        let mut messages = messages?;
        messages.reverse();
        Ok(messages)
    }

    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let sender = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                format!("user:{}", content_message.sender)
            }
            Message::Event(_) => "system".to_string(),
        };
        let sent = match &self.timestamped_message.message {
            Message::Content(content_message) => content_message.sent,
            Message::Event(_) => true,
        };
        let content = self
            .timestamped_message
            .message
            .to_versioned_message()
            .map_err(|error| sqlx::Error::Encode(Box::new(error)))?;

        query!(
            "INSERT INTO conversation_messages
            (message_id, conversation_id, timestamp, sender, content, sent)
            VALUES (?, ?, ?, ?, ?, ?)",
            self.conversation_message_id,
            self.conversation_id,
            self.timestamped_message.timestamp,
            sender,
            content,
            sent,
        )
        .execute(executor)
        .await?;

        notifier
            .add(self.conversation_message_id)
            .add(self.conversation_id);
        Ok(())
    }

    /// Set the message's sent status in the database and update the message's timestamp.
    pub(super) async fn update_sent_status(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        message_id: ConversationMessageId,
        timestamp: TimeStamp,
        sent: bool,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE conversation_messages SET timestamp = ?, sent = ? WHERE message_id = ?",
            timestamp,
            sent,
            message_id,
        )
        .execute(executor)
        .await?;
        notifier.update(message_id);
        Ok(())
    }

    /// Get the last content message in the conversation.
    pub(crate) async fn last_content_message(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ConversationId,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender,
                content AS "content: _",
                sent
            FROM conversation_messages
            WHERE conversation_id = ? AND sender != 'system'
            ORDER BY timestamp DESC LIMIT 1"#,
            conversation_id,
        )
        .fetch_optional(executor)
        .await
        .map(|record| {
            record
                .map(TryFrom::try_from)
                .transpose()
                .map_err(From::from)
        })?
    }

    pub(crate) async fn prev_message(
        executor: impl SqliteExecutor<'_>,
        message_id: ConversationMessageId,
    ) -> sqlx::Result<Option<ConversationMessage>> {
        query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender,
                content AS "content: _",
                sent
            FROM conversation_messages
            WHERE message_id != ?1
                AND timestamp <= (SELECT timestamp FROM conversation_messages
                WHERE message_id = ?1)
            ORDER BY timestamp DESC
            LIMIT 1"#,
            message_id,
        )
        .fetch_optional(executor)
        .await
        .map(|record| {
            record
                .map(TryFrom::try_from)
                .transpose()
                .map_err(From::from)
        })?
    }

    pub(crate) async fn next_message(
        executor: impl SqliteExecutor<'_>,
        message_id: ConversationMessageId,
    ) -> sqlx::Result<Option<ConversationMessage>> {
        query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender,
                content AS "content: _",
                sent
            FROM conversation_messages
            WHERE message_id != ?1
                AND timestamp >= (SELECT timestamp FROM conversation_messages
                WHERE message_id = ?1)
            ORDER BY timestamp ASC
            LIMIT 1"#,
            message_id,
        )
        .fetch_optional(executor)
        .await
        .map(|record| {
            record
                .map(TryFrom::try_from)
                .transpose()
                .map_err(From::from)
        })?
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::Utc;
    use sqlx::SqlitePool;

    use crate::{
        conversations::persistence::tests::test_conversation, EventMessage, MimiContent,
        SystemMessage,
    };

    use super::*;

    pub(crate) fn test_conversation_message(
        conversation_id: ConversationId,
    ) -> ConversationMessage {
        let conversation_message_id = ConversationMessageId::random();
        let timestamp = Utc::now().into();
        let message = Message::Content(Box::new(ContentMessage {
            sender: "alice@localhost".to_string(),
            sent: false,
            content: MimiContent::simple_markdown_message(
                "localhost".parse().unwrap(),
                "Hello world!".to_string(),
            ),
        }));
        let timestamped_message = TimestampedMessage { timestamp, message };
        ConversationMessage {
            conversation_message_id,
            conversation_id,
            timestamped_message,
        }
    }

    #[sqlx::test]
    async fn store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let message = test_conversation_message(conversation.id());

        message.store(&pool, &mut store_notifier).await?;
        let loaded = ConversationMessage::load(&pool, message.id())
            .await?
            .unwrap();
        assert_eq!(loaded, message);

        Ok(())
    }

    #[sqlx::test]
    async fn store_load_multiple(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&pool, &mut store_notifier).await?;
        message_b.store(&pool, &mut store_notifier).await?;

        let loaded = ConversationMessage::load_multiple(&pool, conversation.id(), 2).await?;
        assert_eq!(loaded, [message_a, message_b.clone()]);

        let loaded = ConversationMessage::load_multiple(&pool, conversation.id(), 1).await?;
        assert_eq!(loaded, [message_b]);

        Ok(())
    }

    #[sqlx::test]
    async fn update_sent_status(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let message = test_conversation_message(conversation.id());
        message.store(&pool, &mut store_notifier).await?;

        let loaded = ConversationMessage::load(&pool, message.id())
            .await?
            .unwrap();
        assert!(!loaded.is_sent());

        let sent_at: TimeStamp = Utc::now().into();
        ConversationMessage::update_sent_status(
            &pool,
            &mut store_notifier,
            loaded.id(),
            sent_at,
            true,
        )
        .await?;

        let loaded = ConversationMessage::load(&pool, message.id())
            .await?
            .unwrap();
        assert_eq!(&loaded.timestamp(), sent_at.as_ref());
        assert!(loaded.is_sent());

        Ok(())
    }

    #[sqlx::test]
    async fn last_content_message(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&pool, &mut store_notifier).await?;
        message_b.store(&pool, &mut store_notifier).await?;

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
        .store(&pool, &mut store_notifier)
        .await?;

        let loaded = ConversationMessage::last_content_message(&pool, conversation.id()).await?;
        assert_eq!(loaded, Some(message_b));

        Ok(())
    }

    #[sqlx::test]
    async fn prev_message(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&pool, &mut store_notifier).await?;
        message_b.store(&pool, &mut store_notifier).await?;

        let loaded = ConversationMessage::prev_message(&pool, message_b.id()).await?;
        assert_eq!(loaded, Some(message_a));

        Ok(())
    }

    #[sqlx::test]
    async fn next_message(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation.store(&pool, &mut store_notifier).await?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&pool, &mut store_notifier).await?;
        message_b.store(&pool, &mut store_notifier).await?;

        let loaded = ConversationMessage::next_message(&pool, message_a.id()).await?;
        assert_eq!(loaded, Some(message_b));

        Ok(())
    }
}
