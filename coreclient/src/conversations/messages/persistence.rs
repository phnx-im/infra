// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::bail;
use phnxtypes::{
    codec::{self, PhnxCodec},
    time::TimeStamp,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    encode::IsNull, error::BoxDynError, query, query_as, Database, Decode, Encode, Sqlite,
    SqliteExecutor,
};
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::{
    store::StoreNotifier, ContentMessage, ConversationId, ConversationMessage, Message, MimiContent,
};

use super::{ErrorMessage, EventMessage};

const CURRENT_MESSAGE_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
struct VersionedMessage {
    version: u16,
    // We store the message as bytes, because deserialization depends on
    // other parameters.
    content: Vec<u8>,
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
        Encode::<Sqlite>::encode(bytes, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for VersionedMessage {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let bytes = <&[u8] as Decode<Sqlite>>::decode(value)?;
        Ok(PhnxCodec::from_slice(bytes)?)
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
}

use super::{ConversationMessageId, TimestampedMessage};

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
        let message = match sender.as_str() {
            "system" => Message::Event(content.to_event_message().unwrap_or_else(|e| {
                warn!("Event parsing failed: {e}");
                EventMessage::Error(ErrorMessage::new("Event parsing failed".to_owned()))
            })),
            _ => {
                let sender = sender
                    .strip_prefix("user:")
                    .ok_or_else(|| VersionedMessageError::InvalidUserPrefix)?
                    .to_owned();
                content
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
                    })
            }
        };
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
        let sent = match &self.timestamped_message.message {
            Message::Content(content_message) => content_message.sent,
            Message::Event(_) => true,
        };

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
            .update(self.conversation_id);
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
