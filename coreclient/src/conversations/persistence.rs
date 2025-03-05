// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use openmls::group::GroupId;
use sqlx::{query, query_as, query_scalar, Connection, SqliteExecutor};
use tokio_stream::StreamExt;
use tracing::info;

use crate::{
    store::StoreNotifier,
    utils::persistence::{GroupIdWrapper, Storable},
    Conversation, ConversationAttributes, ConversationId, ConversationMessageId,
    ConversationStatus, ConversationType,
};

impl Storable for Conversation {
    const CREATE_TABLE_STATEMENT: &'static str = "
        CREATE TABLE IF NOT EXISTS conversations (
            conversation_id BLOB PRIMARY KEY,
            conversation_title TEXT NOT NULL,
            conversation_picture BLOB,
            group_id BLOB NOT NULL,
            last_read TEXT NOT NULL,
            conversation_status TEXT NOT NULL CHECK (conversation_status LIKE 'active' OR conversation_status LIKE 'inactive:%'),
            conversation_type TEXT NOT NULL CHECK (conversation_type LIKE 'group' OR conversation_type LIKE 'unconfirmed_connection:%' OR conversation_type LIKE 'connection:%')
        );";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let id = row.get(0)?;
        let title = row.get(1)?;
        let picture = row.get(2)?;
        let group_id: GroupIdWrapper = row.get(3)?;
        let last_read = row.get(4)?;
        let status = row.get(5)?;
        let conversation_type = row.get(6)?;

        Ok(Conversation {
            id,
            group_id: group_id.into(),
            last_read,
            status,
            conversation_type,
            attributes: ConversationAttributes { title, picture },
        })
    }
}

struct SqlConversation {
    conversation_id: ConversationId,
    conversation_title: String,
    conversation_picture: Option<Vec<u8>>,
    group_id: GroupIdWrapper,
    last_read: DateTime<Utc>,
    conversation_status: ConversationStatus,
    conversation_type: ConversationType,
}

impl From<SqlConversation> for Conversation {
    fn from(
        SqlConversation {
            conversation_id,
            conversation_title,
            conversation_picture,
            group_id: GroupIdWrapper(group_id),
            last_read,
            conversation_status,
            conversation_type,
        }: SqlConversation,
    ) -> Self {
        Self {
            id: conversation_id,
            group_id,
            last_read,
            status: conversation_status,
            conversation_type,
            attributes: ConversationAttributes {
                title: conversation_title,
                picture: conversation_picture,
            },
        }
    }
}

impl Conversation {
    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        info!(
            id =% self.id,
            title =% self.attributes().title(),
            "Storing conversation"
        );
        let title = self.attributes().title();
        let picture = self.attributes().picture();
        let group_id = self.group_id.as_slice();
        let conversation_status = self.status();
        let conversation_type = self.conversation_type();
        query!(
            "INSERT INTO conversations (
                conversation_id,
                conversation_title,
                conversation_picture,
                group_id,
                last_read,
                conversation_status,
                conversation_type
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)",
            self.id,
            title,
            picture,
            group_id,
            self.last_read,
            conversation_status,
            conversation_type
        )
        .execute(executor)
        .await?;
        notifier.add(self.id);
        Ok(())
    }

    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        conversation_id: &ConversationId,
    ) -> sqlx::Result<Option<Conversation>> {
        query_as!(
            SqlConversation,
            r#"SELECT
                conversation_id AS "conversation_id: _",
                conversation_title,
                conversation_picture,
                group_id AS "group_id: _",
                last_read AS "last_read: _",
                conversation_status AS "conversation_status: _",
                conversation_type AS "conversation_type: _"
            FROM conversations
            WHERE conversation_id = ?"#,
            conversation_id
        )
        .fetch_optional(executor)
        .await
        .map(|value| value.map(From::from))
    }

    pub(crate) async fn load_by_group_id(
        executor: impl SqliteExecutor<'_>,
        group_id: &GroupId,
    ) -> sqlx::Result<Option<Conversation>> {
        let group_id = group_id.as_slice();
        query_as!(
            SqlConversation,
            r#"SELECT
                conversation_id AS "conversation_id: _",
                conversation_title,
                conversation_picture,
                group_id AS "group_id: _",
                last_read AS "last_read: _",
                conversation_status AS "conversation_status: _",
                conversation_type AS "conversation_type: _"
            FROM conversations WHERE group_id = ?"#,
            group_id
        )
        .fetch_optional(executor)
        .await
        .map(|value| value.map(From::from))
    }

    pub(crate) async fn load_all(
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<Vec<Conversation>> {
        query_as!(
            SqlConversation,
            r#"SELECT
                conversation_id AS "conversation_id: _",
                conversation_title,
                conversation_picture,
                group_id AS "group_id: _",
                last_read AS "last_read: _",
                conversation_status AS "conversation_status: _",
                conversation_type AS "conversation_type: _"
            FROM conversations"#,
        )
        .fetch(executor)
        .map(|res| res.map(From::from))
        .collect()
        .await
    }

    pub(super) async fn update_picture(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        conversation_picture: Option<&[u8]>,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE conversations SET conversation_picture = ? WHERE conversation_id = ?",
            conversation_picture,
            conversation_id,
        )
        .execute(executor)
        .await?;
        notifier.update(conversation_id);
        Ok(())
    }

    pub(super) async fn update_status(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        status: &ConversationStatus,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE conversations SET conversation_status = ? WHERE conversation_id = ?",
            status,
            conversation_id,
        )
        .execute(executor)
        .await?;
        notifier.update(conversation_id);
        Ok(())
    }

    pub(crate) async fn delete(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM conversations WHERE conversation_id = ?",
            conversation_id
        )
        .execute(executor)
        .await?;
        notifier.remove(conversation_id);
        Ok(())
    }

    /// Set the `last_read` marker of all conversations with the given
    /// [`ConversationId`]s to the given timestamps. This is used to mark all
    /// messages up to this timestamp as read.
    pub(crate) async fn mark_as_read(
        connection: &mut sqlx::SqliteConnection,
        notifier: &mut StoreNotifier,
        mark_as_read_data: impl IntoIterator<Item = (ConversationId, DateTime<Utc>)>,
    ) -> sqlx::Result<()> {
        let mut transaction = connection.begin().await?;

        for (conversation_id, timestamp) in mark_as_read_data {
            let unread_messages: Vec<ConversationMessageId> = query_scalar!(
                r#"SELECT
                    message_id AS "message_id: _"
                FROM conversation_messages
                INNER JOIN conversations c ON c.conversation_id = :conversation_id
                WHERE c.conversation_id = :conversation_id AND timestamp > c.last_read"#,
                conversation_id,
            )
            .fetch_all(&mut *transaction)
            .await?;

            for message_id in unread_messages {
                notifier.update(message_id);
            }

            let updated = query!(
                "UPDATE conversations
                SET last_read = :timestamp
                WHERE conversation_id = :conversation_id AND last_read < :timestamp",
                timestamp,
                conversation_id,
            )
            .execute(&mut *transaction)
            .await?;
            if updated.rows_affected() == 1 {
                notifier.update(conversation_id);
            }
        }

        transaction.commit().await?;
        Ok(())
    }

    /// Mark all messages in the conversation as read until including the given message id.
    pub(crate) async fn mark_as_read_until_message_id(
        connection: &mut sqlx::SqliteConnection,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        until_message_id: ConversationMessageId,
    ) -> sqlx::Result<bool> {
        let timestamp: Option<DateTime<Utc>> = query_scalar!(
            r#"SELECT
                timestamp AS "timestamp: _"
            FROM conversation_messages WHERE message_id = ?"#,
            until_message_id
        )
        .fetch_optional(&mut *connection)
        .await?;

        let Some(timestamp) = timestamp else {
            return Ok(false);
        };
        let updated = query!(
            "UPDATE conversations SET last_read = :timestamp
            WHERE conversation_id = :conversation_id AND last_read != :timestamp",
            conversation_id,
            timestamp,
        )
        .execute(connection)
        .await?;
        let marked_as_read = updated.rows_affected() == 1;
        if marked_as_read {
            notifier.update(conversation_id);
        }
        Ok(marked_as_read)
    }

    pub(crate) async fn global_unread_message_count(
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<usize> {
        query_scalar!(
            r#"SELECT
                COUNT(cm.conversation_id) AS "count: _"
            FROM
                conversations c
            LEFT JOIN
                conversation_messages cm
            ON
                c.conversation_id = cm.conversation_id
                AND cm.sender != 'system'
                AND cm.timestamp > c.last_read"#
        )
        .fetch_one(executor)
        .await
        .map(|n: u32| n.try_into().expect("usize overflow"))
    }

    pub(crate) async fn messages_count(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ConversationId,
    ) -> sqlx::Result<usize> {
        query_scalar!(
            r#"SELECT
                COUNT(*) AS "count: _"
            FROM
                conversation_messages cm
            WHERE
                cm.conversation_id = :conversation_id
                AND cm.sender != 'system'"#,
            conversation_id
        )
        .fetch_one(executor)
        .await
        .map(|n: u32| n.try_into().expect("usize overflow"))
    }

    pub(crate) async fn unread_messages_count(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ConversationId,
    ) -> sqlx::Result<usize> {
        query_scalar!(
            r#"SELECT
                COUNT(*) AS "count: _"
            FROM
                conversation_messages
            WHERE
                conversation_id = :conversation_id
                AND sender != 'system'
                AND timestamp >
                (
                    SELECT
                        last_read
                    FROM
                        conversations
                    WHERE
                        conversation_id = :conversation_id
                )"#,
            conversation_id
        )
        .fetch_one(executor)
        .await
        .map(|n: u32| n.try_into().expect("usize overflow"))
    }

    pub(super) async fn set_conversation_type(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_type: &ConversationType,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE conversations SET conversation_type = ? WHERE conversation_id = ?",
            conversation_type,
            self.id,
        )
        .execute(executor)
        .await?;
        notifier.update(self.id);
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use chrono::Duration;
    use uuid::Uuid;

    use crate::{
        conversations::messages::persistence::tests::{test_connection, test_conversation_message},
        InactiveConversation,
    };

    use super::*;

    pub(crate) fn test_conversation() -> Conversation {
        let id = ConversationId {
            uuid: Uuid::new_v4(),
        };
        Conversation {
            id,
            group_id: GroupId::from_slice(&[0; 32]),
            last_read: Utc::now(),
            status: ConversationStatus::Active,
            conversation_type: ConversationType::Group,
            attributes: ConversationAttributes {
                title: "Test conversation".to_string(),
                picture: None,
            },
        }
    }

    #[test]
    fn store_load() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(&connection, &mut store_notifier)
            .unwrap();
        let loaded =
            Conversation::load(&connection, &conversation.id)?.expect("missing conversation");
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[test]
    fn store_load_by_group_id() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(&connection, &mut store_notifier)
            .unwrap();
        let loaded = Conversation::load_by_group_id(&connection, &conversation.group_id)?
            .expect("missing conversation");
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[test]
    fn store_load_all() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation_a = test_conversation();
        conversation_a
            .store(&connection, &mut store_notifier)
            .unwrap();

        let conversation_b = test_conversation();
        conversation_b
            .store(&connection, &mut store_notifier)
            .unwrap();

        let loaded = Conversation::load_all(&connection)?;
        assert_eq!(loaded, [conversation_a, conversation_b]);

        Ok(())
    }

    #[test]
    fn update_conversation_picture() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let mut conversation = test_conversation();
        conversation
            .store(&connection, &mut store_notifier)
            .unwrap();

        let new_picture = [1, 2, 3];
        Conversation::update_picture(
            &connection,
            &mut store_notifier,
            conversation.id,
            Some(&new_picture),
        )?;

        conversation.attributes.picture = Some(new_picture.to_vec());

        let loaded = Conversation::load(&connection, &conversation.id)?.unwrap();
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[test]
    fn update_conversation_status() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let mut conversation = test_conversation();
        conversation
            .store(&connection, &mut store_notifier)
            .unwrap();

        let past_members = vec![
            "alice@localhost".parse().unwrap(),
            "bob@localhost".parse().unwrap(),
        ];
        let status = ConversationStatus::Inactive(InactiveConversation::new(past_members));
        Conversation::update_status(&connection, &mut store_notifier, conversation.id, &status)?;

        conversation.status = status;
        let loaded = Conversation::load(&connection, &conversation.id)?.unwrap();
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[test]
    fn delete() -> anyhow::Result<()> {
        let connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(&connection, &mut store_notifier)
            .unwrap();
        let loaded = Conversation::load(&connection, &conversation.id)?.unwrap();
        assert_eq!(loaded, conversation);

        Conversation::delete(&connection, &mut store_notifier, conversation.id)?;
        let loaded = Conversation::load(&connection, &conversation.id)?;
        assert!(loaded.is_none());

        Ok(())
    }

    #[test]
    fn counters() -> anyhow::Result<()> {
        let mut connection = test_connection();
        let mut store_notifier = StoreNotifier::noop();

        let conversation_a = test_conversation();
        conversation_a.store(&connection, &mut store_notifier)?;

        let conversation_b = test_conversation();
        conversation_b.store(&connection, &mut store_notifier)?;

        let message_a = test_conversation_message(conversation_a.id());
        let message_b = test_conversation_message(conversation_b.id());

        message_a.store(&connection, &mut store_notifier)?;
        message_b.store(&connection, &mut store_notifier)?;

        let n = Conversation::messages_count(&connection, conversation_a.id())?;
        assert_eq!(n, 1);

        let n = Conversation::messages_count(&connection, conversation_b.id())?;
        assert_eq!(n, 1);

        let n = Conversation::global_unread_message_count(&connection)?;
        assert_eq!(n, 2);

        let transaction = connection.transaction()?;
        Conversation::mark_as_read(
            &transaction,
            &mut store_notifier,
            [(
                conversation_a.id(),
                message_a.timestamp() - Duration::seconds(1),
            )],
        )?;
        transaction.commit()?;
        let n = Conversation::unread_messages_count(&connection, conversation_a.id())?;
        assert_eq!(n, 1);

        let transaction = connection.transaction()?;
        Conversation::mark_as_read(
            &transaction,
            &mut store_notifier,
            [(conversation_a.id(), Utc::now())],
        )?;
        transaction.commit()?;
        let n = Conversation::unread_messages_count(&connection, conversation_a.id())?;
        assert_eq!(n, 0);

        Conversation::mark_as_read_until_message_id(
            &connection,
            &mut store_notifier,
            conversation_b.id(),
            ConversationMessageId::random(),
        )?;
        let n = Conversation::unread_messages_count(&connection, conversation_b.id())?;
        assert_eq!(n, 1);

        Conversation::mark_as_read_until_message_id(
            &connection,
            &mut store_notifier,
            conversation_b.id(),
            message_b.id(),
        )?;
        let n = Conversation::unread_messages_count(&connection, conversation_b.id())?;
        assert_eq!(n, 0);

        let n = Conversation::global_unread_message_count(&connection)?;
        assert_eq!(n, 0);

        Ok(())
    }
}
