// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::{
    codec::{self, BlobDecoded, BlobEncoded, PersistenceCodec},
    identifiers::{Fqdn, MimiId, UserId},
    time::TimeStamp,
};
use anyhow::bail;
use mimi_content::{MessageStatus, MimiContent};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteExecutor, query, query_as};
use tokio_stream::StreamExt;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{ContentMessage, ConversationId, ConversationMessage, Message, store::StoreNotifier};

use super::{ErrorMessage, EventMessage};

const UNKNOWN_MESSAGE_VERSION: u16 = 0;
const CURRENT_MESSAGE_VERSION: u16 = 1;

#[derive(Serialize, Deserialize)]
pub(crate) struct VersionedMessage {
    #[serde(default = "VersionedMessage::unknown_message_version")]
    pub(crate) version: u16,
    // We store the message as bytes, because deserialization depends on
    // other parameters.
    #[serde(default)]
    pub(crate) content: Vec<u8>,
}

impl VersionedMessage {
    const fn unknown_message_version() -> u16 {
        UNKNOWN_MESSAGE_VERSION
    }
}

impl VersionedMessage {
    fn to_event_message(&self) -> anyhow::Result<EventMessage> {
        match self.version {
            CURRENT_MESSAGE_VERSION => {
                Ok(PersistenceCodec::from_slice::<EventMessage>(&self.content)?)
            }
            other => bail!("unknown event message version: {other}"),
        }
    }

    pub(crate) fn to_mimi_content(&self) -> anyhow::Result<MimiContent> {
        match self.version {
            CURRENT_MESSAGE_VERSION => {
                Ok(PersistenceCodec::from_slice::<MimiContent>(&self.content)?)
            }
            other => bail!("unknown mimi content message version: {other}"),
        }
    }

    fn from_event_message(
        event: &EventMessage,
    ) -> Result<VersionedMessage, aircommon::codec::Error> {
        Ok(VersionedMessage {
            version: CURRENT_MESSAGE_VERSION,
            content: PersistenceCodec::to_vec(&event)?,
        })
    }

    pub(crate) fn from_mimi_content(
        content: &MimiContent,
    ) -> Result<VersionedMessage, aircommon::codec::Error> {
        Ok(VersionedMessage {
            version: CURRENT_MESSAGE_VERSION,
            content: PersistenceCodec::to_vec(&content)?,
        })
    }
}

use super::{ConversationMessageId, TimestampedMessage};

struct SqlConversationMessage {
    message_id: ConversationMessageId,
    mimi_id: Option<MimiId>,
    conversation_id: ConversationId,
    timestamp: TimeStamp,
    sender_user_uuid: Option<Uuid>,
    sender_user_domain: Option<Fqdn>,
    content: BlobDecoded<VersionedMessage>,
    sent: bool,
    status: i64,
    edited_at: Option<TimeStamp>,
}

#[derive(thiserror::Error, Debug)]
enum VersionedMessageError {
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
            mimi_id,
            conversation_id,
            timestamp,
            sender_user_uuid,
            sender_user_domain,
            content,
            sent,
            status,
            edited_at,
        }: SqlConversationMessage,
    ) -> Result<Self, Self::Error> {
        let message = match (sender_user_uuid, sender_user_domain) {
            // user message
            (Some(sender_user_uuid), Some(sender_user_domain)) => {
                let sender = UserId::new(sender_user_uuid, sender_user_domain);
                content
                    .into_inner()
                    .to_mimi_content()
                    .map(|content| {
                        Message::Content(Box::new(ContentMessage {
                            sender,
                            sent,
                            content,
                            mimi_id,
                            edited_at,
                        }))
                    })
                    .unwrap_or_else(|e| {
                        warn!("Message parsing failed: {e}");
                        Message::Event(EventMessage::Error(ErrorMessage::new(
                            "Message parsing failed".to_owned(),
                        )))
                    })
            }
            // system message
            _ => Message::Event(content.into_inner().to_event_message().unwrap_or_else(|e| {
                warn!("Event parsing failed: {e}");
                EventMessage::Error(ErrorMessage::new("Event parsing failed".to_owned()))
            })),
        };

        let timestamped_message = TimestampedMessage { timestamp, message };
        Ok(ConversationMessage {
            conversation_message_id: message_id,
            conversation_id,
            timestamped_message,
            status: u8::try_from(status).map_or(MessageStatus::Unread, MessageStatus::from_repr),
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
                mimi_id AS "mimi_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
            FROM conversation_messages
            WHERE message_id = ?
            "#,
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

    pub(crate) async fn load_by_mimi_id(
        executor: impl SqliteExecutor<'_>,
        mimi_id: &MimiId,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                mimi_id AS "mimi_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
            FROM conversation_messages
            WHERE mimi_id = ?
            "#,
            mimi_id,
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
                mimi_id AS "mimi_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
            FROM conversation_messages
            WHERE conversation_id = ?
            ORDER BY timestamp DESC
            LIMIT ?"#,
            conversation_id,
            number_of_messages,
        )
        .fetch(executor)
        .filter_map(|res| {
            let message: sqlx::Result<ConversationMessage> = res
                // skip messages that we can't decode, but don't fail loading the rest of the
                // messages
                .inspect_err(|e| warn!("Error loading message: {e}"))
                .ok()?
                .try_into()
                .map_err(From::from);
            Some(message)
        })
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
        let (sender_uuid, sender_domain, mimi_id) = match &self.timestamped_message.message {
            Message::Content(content_message) => (
                Some(content_message.sender.uuid()),
                Some(content_message.sender.domain()),
                Some(content_message.mimi_id()),
            ),
            Message::Event(_) => (None, None, None),
        };
        let content = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                VersionedMessage::from_mimi_content(&content_message.content)?
            }
            Message::Event(event_message) => VersionedMessage::from_event_message(event_message)?,
        };
        let content = BlobEncoded(&content);
        let sent = match &self.timestamped_message.message {
            Message::Content(content_message) => content_message.sent,
            Message::Event(_) => true,
        };

        query!(
            "INSERT INTO conversation_messages (
                message_id,
                mimi_id,
                conversation_id,
                timestamp,
                sender_user_uuid,
                sender_user_domain,
                content,
                sent
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            self.conversation_message_id,
            mimi_id,
            self.conversation_id,
            self.timestamped_message.timestamp,
            sender_uuid,
            sender_domain,
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

    pub(crate) async fn update(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> anyhow::Result<()> {
        let mimi_id = self.message().mimi_id();
        let content = match &self.timestamped_message.message {
            Message::Content(content_message) => {
                VersionedMessage::from_mimi_content(&content_message.content)?
            }
            Message::Event(event_message) => VersionedMessage::from_event_message(event_message)?,
        };
        let content = BlobEncoded(&content);
        let sent = match &self.timestamped_message.message {
            Message::Content(content_message) => content_message.sent,
            Message::Event(_) => true,
        };
        let edited_at = self.edited_at();
        let status = self.status().repr();
        let message_id = self.id();

        query!(
            "UPDATE conversation_messages
            SET
                mimi_id = ?,
                timestamp = ?,
                content = ?,
                sent = ?,
                edited_at = ?,
                status = ?
            WHERE message_id = ?",
            mimi_id,
            self.timestamped_message.timestamp,
            content,
            sent,
            edited_at,
            status,
            message_id,
        )
        .execute(executor)
        .await?;
        notifier.update(self.id());
        notifier.update(self.conversation_id);
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
        let res = query!(
            "UPDATE conversation_messages SET timestamp = ?, sent = ? WHERE message_id = ?",
            timestamp,
            sent,
            message_id,
        )
        .execute(executor)
        .await?;
        if res.rows_affected() == 1 {
            notifier.update(message_id);
        }
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
                mimi_id AS "mimi_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
            FROM conversation_messages
            WHERE conversation_id = ?
                AND sender_user_uuid IS NOT NULL
                AND sender_user_domain IS NOT NULL
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

    /// Get the last content message in the conversation which is owned by the given user.
    pub(crate) async fn last_content_message_by_user(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ConversationId,
        user_id: &UserId,
    ) -> sqlx::Result<Option<Self>> {
        let user_uuid = user_id.uuid();
        let user_domain = user_id.domain();
        query_as!(
            SqlConversationMessage,
            r#"SELECT
                message_id AS "message_id: _",
                conversation_id AS "conversation_id: _",
                mimi_id AS "mimi_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
            FROM conversation_messages
            WHERE conversation_id = ?
                AND sender_user_uuid = ?
                AND sender_user_domain = ?
            ORDER BY timestamp DESC LIMIT 1"#,
            conversation_id,
            user_uuid,
            user_domain,
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
                mimi_id AS "mimi_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
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
                mimi_id AS "mimi_id: _",
                conversation_id AS "conversation_id: _",
                timestamp AS "timestamp: _",
                sender_user_uuid AS "sender_user_uuid: _",
                sender_user_domain AS "sender_user_domain: _",
                content AS "content: _",
                sent,
                status,
                edited_at AS "edited_at: _"
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
    use std::sync::LazyLock;

    use chrono::Utc;
    use openmls::group::GroupId;
    use sqlx::SqlitePool;

    use crate::{
        EventMessage, SystemMessage, conversations::persistence::tests::test_conversation,
    };

    use super::*;

    pub(crate) fn test_conversation_message(
        conversation_id: ConversationId,
    ) -> ConversationMessage {
        test_conversation_message_with_salt(conversation_id, [0; 16])
    }

    pub(crate) fn test_conversation_message_with_salt(
        conversation_id: ConversationId,
        salt: [u8; 16],
    ) -> ConversationMessage {
        let conversation_message_id = ConversationMessageId::random();
        let timestamp = Utc::now().into();
        let message = Message::Content(Box::new(ContentMessage::new(
            UserId::random("localhost".parse().unwrap()),
            false,
            MimiContent::simple_markdown_message("Hello world!".to_string(), salt),
            &GroupId::from_slice(&[0]),
        )));
        let timestamped_message = TimestampedMessage { timestamp, message };
        ConversationMessage {
            conversation_message_id,
            conversation_id,
            timestamped_message,
            status: MessageStatus::Unread,
        }
    }

    #[sqlx::test]
    async fn store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

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
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

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
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

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
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

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
                    UserId::random("localhost".parse()?),
                    UserId::random("localhost".parse()?),
                ))),
            },
            status: MessageStatus::Unread,
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
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

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
        conversation
            .store(pool.acquire().await?.as_mut(), &mut store_notifier)
            .await?;

        let message_a = test_conversation_message(conversation.id());
        let message_b = test_conversation_message(conversation.id());

        message_a.store(&pool, &mut store_notifier).await?;
        message_b.store(&pool, &mut store_notifier).await?;

        let loaded = ConversationMessage::next_message(&pool, message_a.id()).await?;
        assert_eq!(loaded, Some(message_b));

        Ok(())
    }

    static VERSIONED_MESSAGE: LazyLock<VersionedMessage> = LazyLock::new(|| {
        VersionedMessage::from_mimi_content(&MimiContent::simple_markdown_message(
            "Hello world!".to_string(),
            [0; 16], // simple salt for testing
        ))
        .unwrap()
    });

    #[test]
    fn versioned_message_serde_codec() {
        insta::assert_binary_snapshot!(
            ".cbor",
            PersistenceCodec::to_vec(&*VERSIONED_MESSAGE).unwrap()
        );
    }

    #[test]
    fn versioned_message_serde_json() {
        insta::assert_json_snapshot!(&*VERSIONED_MESSAGE);
    }
}
