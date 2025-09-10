// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::{
    identifiers::{Fqdn, MimiId, UserHandle, UserId},
    time::TimeStamp,
};
use chrono::{DateTime, Utc};
use mimi_content::MessageStatus;
use openmls::group::GroupId;
use sqlx::{
    Connection, SqliteConnection, SqliteExecutor, SqliteTransaction, query, query_as, query_scalar,
};
use tokio_stream::StreamExt;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    Chat, ChatAttributes, ChatId, ChatStatus, ChatType, MessageId, store::StoreNotifier,
    utils::persistence::GroupIdWrapper,
};

use super::InactiveChat;

struct SqlConversation {
    conversation_id: ChatId,
    conversation_title: String,
    conversation_picture: Option<Vec<u8>>,
    group_id: GroupIdWrapper,
    last_read: DateTime<Utc>,
    connection_user_uuid: Option<Uuid>,
    connection_user_domain: Option<Fqdn>,
    connection_user_handle: Option<UserHandle>,
    is_confirmed_connection: bool,
    is_active: bool,
}

impl SqlConversation {
    fn convert(self, past_members: Vec<SqlPastMember>) -> Option<Chat> {
        let Self {
            conversation_id,
            conversation_title,
            conversation_picture,
            group_id: GroupIdWrapper(group_id),
            last_read,
            connection_user_uuid,
            connection_user_domain,
            connection_user_handle,
            is_confirmed_connection,
            is_active,
        } = self;

        let conversation_type = match (
            connection_user_uuid,
            connection_user_domain,
            connection_user_handle,
        ) {
            (Some(user_uuid), Some(domain), _) => {
                let connection_user_id = UserId::new(user_uuid, domain);
                if is_confirmed_connection {
                    ChatType::Connection(connection_user_id)
                } else {
                    warn!("Unconfirmed user connections are not supported anymore");
                    return None;
                }
            }

            (None, None, Some(handle)) => ChatType::HandleConnection(handle),
            _ => ChatType::Group,
        };

        let status = if is_active {
            ChatStatus::Active
        } else {
            ChatStatus::Inactive(InactiveChat::new(
                past_members.into_iter().map(From::from).collect(),
            ))
        };

        Some(Chat {
            id: conversation_id,
            group_id,
            last_read,
            status,
            conversation_type,
            attributes: ChatAttributes {
                title: conversation_title,
                picture: conversation_picture,
            },
        })
    }

    async fn load_past_members(
        &self,
        connection: &mut SqliteConnection,
    ) -> sqlx::Result<Vec<SqlPastMember>> {
        if self.is_active {
            return Ok(Vec::new());
        }
        Chat::load_past_members(connection, self.conversation_id).await
    }
}

struct SqlPastMember {
    member_user_uuid: Uuid,
    member_user_domain: Fqdn,
}

impl From<SqlPastMember> for UserId {
    fn from(
        SqlPastMember {
            member_user_uuid,
            member_user_domain,
        }: SqlPastMember,
    ) -> Self {
        UserId::new(member_user_uuid, member_user_domain)
    }
}

impl Chat {
    pub(crate) async fn store(
        &self,
        connection: &mut SqliteConnection,
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
        let (is_active, past_members) = match self.status() {
            ChatStatus::Inactive(inactive_conversation) => {
                (false, inactive_conversation.past_members().to_vec())
            }
            ChatStatus::Active => (true, Vec::new()),
        };
        let (
            is_confirmed_connection,
            connection_user_uuid,
            connection_user_domain,
            connection_user_handle,
        ) = match self.chat_type() {
            ChatType::HandleConnection(handle) => (false, None, None, Some(handle)),
            ChatType::Connection(user_id) => (
                true,
                Some(user_id.uuid()),
                Some(user_id.domain().clone()),
                None,
            ),
            ChatType::Group => (true, None, None, None),
        };
        query!(
            "INSERT INTO conversation (
                conversation_id,
                conversation_title,
                conversation_picture,
                group_id,
                last_read,
                connection_user_uuid,
                connection_user_domain,
                connection_user_handle,
                is_confirmed_connection,
                is_active
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            self.id,
            title,
            picture,
            group_id,
            self.last_read,
            connection_user_uuid,
            connection_user_domain,
            connection_user_handle,
            is_confirmed_connection,
            is_active,
        )
        .execute(&mut *connection)
        .await?;

        for member in past_members {
            let (uuid, domain) = member.into_parts();
            query!(
                "INSERT OR IGNORE INTO conversation_past_member (
                    conversation_id,
                    member_user_uuid,
                    member_user_domain
                )
                VALUES (?, ?, ?)",
                self.id,
                uuid,
                domain,
            )
            .execute(&mut *connection)
            .await?;
        }

        notifier.add(self.id);
        Ok(())
    }

    pub(crate) async fn load(
        connection: &mut SqliteConnection,
        chat_id: &ChatId,
    ) -> sqlx::Result<Option<Chat>> {
        let mut transaction = connection.begin().await?;
        let conversation = query_as!(
            SqlConversation,
            r#"SELECT
                conversation_id AS "conversation_id: _",
                conversation_title,
                conversation_picture,
                group_id AS "group_id: _",
                last_read AS "last_read: _",
                connection_user_uuid AS "connection_user_uuid: _",
                connection_user_domain AS "connection_user_domain: _",
                connection_user_handle AS "connection_user_handle: _",
                is_confirmed_connection,
                is_active
            FROM conversation
            WHERE conversation_id = ?"#,
            chat_id
        )
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(conversation) = conversation else {
            return Ok(None);
        };
        let members = conversation.load_past_members(&mut transaction).await?;
        transaction.commit().await?;
        Ok(conversation.convert(members))
    }

    pub(crate) async fn load_by_group_id(
        connection: &mut SqliteConnection,
        group_id: &GroupId,
    ) -> sqlx::Result<Option<Chat>> {
        let group_id = group_id.as_slice();
        let mut transaction = connection.begin().await?;
        let conversation = query_as!(
            SqlConversation,
            r#"SELECT
                conversation_id AS "conversation_id: _",
                conversation_title,
                conversation_picture,
                group_id AS "group_id: _",
                last_read AS "last_read: _",
                connection_user_uuid AS "connection_user_uuid: _",
                connection_user_domain AS "connection_user_domain: _",
                connection_user_handle AS "connection_user_handle: _",
                is_confirmed_connection,
                is_active
            FROM conversation WHERE group_id = ?"#,
            group_id
        )
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(conversation) = conversation else {
            return Ok(None);
        };
        let members = conversation.load_past_members(&mut transaction).await?;
        transaction.commit().await?;
        Ok(conversation.convert(members))
    }

    pub(crate) async fn load_all(connection: &mut SqliteConnection) -> sqlx::Result<Vec<Chat>> {
        let mut transaction = connection.begin().await?;
        let mut conversations = query_as!(
            SqlConversation,
            r#"SELECT
                conversation_id AS "conversation_id: _",
                conversation_title,
                conversation_picture,
                group_id AS "group_id: _",
                last_read AS "last_read: _",
                connection_user_uuid AS "connection_user_uuid: _",
                connection_user_domain AS "connection_user_domain: _",
                connection_user_handle AS "connection_user_handle: _",
                is_confirmed_connection,
                is_active
            FROM conversation"#,
        )
        .fetch(&mut *transaction)
        .filter_map(|res| {
            res.map(|conversation| conversation.convert(Vec::new()))
                .transpose()
        })
        .collect::<sqlx::Result<Vec<Chat>>>()
        .await?;
        for conversation in &mut conversations {
            let id = conversation.id();
            if let ChatStatus::Inactive(inactive) = conversation.status_mut() {
                let members = Chat::load_past_members(&mut *transaction, id).await?;
                inactive
                    .past_members_mut()
                    .extend(members.into_iter().map(From::from));
            }
        }
        transaction.commit().await?;
        Ok(conversations)
    }

    pub(super) async fn update_picture(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ChatId,
        conversation_picture: Option<&[u8]>,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE conversation SET conversation_picture = ? WHERE conversation_id = ?",
            conversation_picture,
            conversation_id,
        )
        .execute(executor)
        .await?;
        notifier.update(conversation_id);
        Ok(())
    }

    pub(super) async fn update_status(
        connection: &mut SqliteConnection,
        notifier: &mut StoreNotifier,
        conversation_id: ChatId,
        status: &ChatStatus,
    ) -> sqlx::Result<()> {
        let mut transaction = connection.begin().await?;
        match status {
            ChatStatus::Inactive(inactive) => {
                query!(
                    "UPDATE conversation SET is_active = false WHERE conversation_id = ?",
                    conversation_id,
                )
                .execute(&mut *transaction)
                .await?;
                query!(
                    "DELETE FROM conversation_past_member WHERE conversation_id = ?",
                    conversation_id,
                )
                .execute(&mut *transaction)
                .await?;
                for member in inactive.past_members() {
                    let uuid = member.uuid();
                    let domain = member.domain();
                    query!(
                        "INSERT OR IGNORE INTO conversation_past_member (
                            conversation_id,
                            member_user_uuid,
                            member_user_domain
                        )
                        VALUES (?, ?, ?)",
                        conversation_id,
                        uuid,
                        domain,
                    )
                    .execute(&mut *transaction)
                    .await?;
                }
            }
            ChatStatus::Active => {
                query!(
                    "UPDATE conversation SET is_active = true WHERE conversation_id = ?",
                    conversation_id,
                )
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        notifier.update(conversation_id);
        Ok(())
    }

    pub(crate) async fn delete(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ChatId,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM conversation WHERE conversation_id = ?",
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
        mark_as_read_data: impl IntoIterator<Item = (ChatId, DateTime<Utc>)>,
    ) -> sqlx::Result<()> {
        let mut transaction = connection.begin().await?;

        for (conversation_id, timestamp) in mark_as_read_data {
            let unread_messages: Vec<MessageId> = query_scalar!(
                r#"SELECT
                    message_id AS "message_id: _"
                FROM message
                INNER JOIN conversation c ON c.conversation_id = ?1
                WHERE c.conversation_id = ?1 AND timestamp > c.last_read"#,
                conversation_id,
            )
            .fetch_all(&mut *transaction)
            .await?;

            for message_id in unread_messages {
                notifier.update(message_id);
            }

            let updated = query!(
                "UPDATE conversation
                SET last_read = ?1
                WHERE conversation_id = ?2 AND last_read < ?1",
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
    ///
    /// Returns whether the conversation was marked as read and the mimi ids of the messages that
    /// were marked as read.
    pub(crate) async fn mark_as_read_until_message_id(
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ChatId,
        until_message_id: MessageId,
        own_user: &UserId,
    ) -> sqlx::Result<(bool, Vec<MimiId>)> {
        let (our_user_uuid, our_user_domain) = own_user.clone().into_parts();

        let timestamp: Option<DateTime<Utc>> = query_scalar!(
            r#"SELECT
                timestamp AS "timestamp: _"
            FROM message WHERE message_id = ?"#,
            until_message_id
        )
        .fetch_optional(txn.as_mut())
        .await?;

        let Some(timestamp) = timestamp else {
            return Ok((false, Vec::new()));
        };

        let old_timestamp = query!(
            "SELECT last_read FROM conversation
            WHERE conversation_id = ?",
            conversation_id,
        )
        .fetch_one(txn.as_mut())
        .await?
        .last_read;

        let unread_status = MessageStatus::Unread.repr();
        let delivered_status = MessageStatus::Delivered.repr();
        let new_marked_as_read: Vec<MimiId> = query_scalar!(
            r#"SELECT
                m.mimi_id AS "mimi_id!: _"
            FROM message m
            LEFT JOIN message_status s
                ON s.message_id = m.message_id
                AND s.sender_user_uuid = ?2
                AND s.sender_user_domain = ?3
            WHERE conversation_id = ?1
                AND m.timestamp > ?2
                AND (m.sender_user_uuid != ?3 OR m.sender_user_domain != ?4)
                AND mimi_id IS NOT NULL
                AND (s.status IS NULL OR s.status = ?5 OR s.status = ?6)"#,
            conversation_id,
            old_timestamp,
            our_user_uuid,
            our_user_domain,
            unread_status,
            delivered_status,
        )
        .fetch_all(txn.as_mut())
        .await?;

        let updated = query!(
            "UPDATE conversation SET last_read = ?1
            WHERE conversation_id = ?2 AND last_read != ?1",
            timestamp,
            conversation_id,
        )
        .execute(txn.as_mut())
        .await?;

        let marked_as_read = updated.rows_affected() == 1;
        if marked_as_read {
            notifier.update(conversation_id);
        }
        Ok((marked_as_read, new_marked_as_read))
    }

    pub(crate) async fn mark_as_unread(
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ChatId,
        message_id: MessageId,
    ) -> sqlx::Result<()> {
        let timestamp: Option<TimeStamp> = query_scalar!(
            r#"SELECT
                timestamp AS "timestamp: _"
            FROM message
            WHERE timestamp < (
                SELECT timestamp
                FROM message
                WHERE message_id = ?
            )
            ORDER BY timestamp DESC
            LIMIT 1"#,
            message_id
        )
        .fetch_optional(txn.as_mut())
        .await?;

        query!(
            "UPDATE conversation SET last_read = ?1
            WHERE conversation_id = ?2",
            timestamp,
            conversation_id,
        )
        .execute(txn.as_mut())
        .await?;

        notifier.update(message_id);

        Ok(())
    }

    pub(crate) async fn global_unread_message_count(
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<usize> {
        query_scalar!(
            r#"SELECT
                COUNT(m.conversation_id) AS "count: _"
            FROM
                conversation c
            LEFT JOIN
                message m
            ON
                c.conversation_id = m.conversation_id
                AND m.sender_user_uuid IS NOT NULL
                AND m.sender_user_domain IS NOT NULL
                AND m.timestamp > c.last_read"#
        )
        .fetch_one(executor)
        .await
        .map(|n: u32| n.try_into().expect("usize overflow"))
    }

    pub(crate) async fn messages_count(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ChatId,
    ) -> sqlx::Result<usize> {
        query_scalar!(
            r#"SELECT
                COUNT(*) AS "count: _"
            FROM
                message m
            WHERE
                m.conversation_id = ?
                AND m.sender_user_uuid IS NOT NULL
                AND m.sender_user_domain IS NOT NULL"#,
            conversation_id
        )
        .fetch_one(executor)
        .await
        .map(|n: u32| n.try_into().expect("usize overflow"))
    }

    pub(crate) async fn unread_messages_count(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ChatId,
    ) -> sqlx::Result<usize> {
        query_scalar!(
            r#"SELECT
                COUNT(*) AS "count: _"
            FROM
                message
            WHERE
                conversation_id = ?1
                AND sender_user_uuid IS NOT NULL
                AND sender_user_domain IS NOT NULL
                AND timestamp >
                (
                    SELECT
                        last_read
                    FROM
                        conversation
                    WHERE
                        conversation_id = ?1
                )"#,
            conversation_id
        )
        .fetch_one(executor)
        .await
        .map(|n: u32| n.try_into().expect("usize overflow"))
    }

    pub(super) async fn set_chat_type(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        conversation_type: &ChatType,
    ) -> sqlx::Result<()> {
        match conversation_type {
            ChatType::HandleConnection(handle) => {
                query!(
                    "UPDATE conversation SET
                        connection_user_uuid = NULL,
                        connection_user_domain = NULL,
                        connection_user_handle = ?,
                        is_confirmed_connection = false
                    WHERE conversation_id = ?",
                    handle,
                    self.id,
                )
                .execute(executor)
                .await?;
            }
            ChatType::Connection(user_id) => {
                let uuid = user_id.uuid();
                let domain = user_id.domain();
                query!(
                    "UPDATE conversation SET
                        connection_user_uuid = ?,
                        connection_user_domain = ?,
                        is_confirmed_connection = true
                    WHERE conversation_id = ?",
                    uuid,
                    domain,
                    self.id,
                )
                .execute(executor)
                .await?;
            }
            ChatType::Group => {
                query!(
                    "UPDATE conversation SET
                        connection_user_uuid = NULL,
                        connection_user_domain = NULL
                    WHERE conversation_id = ?",
                    self.id,
                )
                .execute(executor)
                .await?;
            }
        }
        notifier.update(self.id);
        Ok(())
    }

    async fn load_past_members(
        executor: impl SqliteExecutor<'_>,
        conversation_id: ChatId,
    ) -> sqlx::Result<Vec<SqlPastMember>> {
        let mut members = query_as!(
            SqlPastMember,
            r#"SELECT
                member_user_uuid AS "member_user_uuid: _",
                member_user_domain AS "member_user_domain: _"
            FROM conversation_past_member
            WHERE conversation_id = ?"#,
            conversation_id
        )
        .fetch_all(executor)
        .await?;
        // make the order deterministic
        members.sort_unstable_by(|a, b| {
            a.member_user_uuid
                .cmp(&b.member_user_uuid)
                .then(a.member_user_domain.cmp(&b.member_user_domain))
        });
        Ok(members)
    }
}

#[cfg(test)]
pub mod tests {
    use chrono::Duration;
    use sqlx::{Sqlite, pool::PoolConnection};
    use uuid::Uuid;

    use crate::{
        InactiveChat, conversations::messages::persistence::tests::test_conversation_message,
    };

    use super::*;

    pub(crate) fn test_chat() -> Chat {
        let id = ChatId {
            uuid: Uuid::new_v4(),
        };
        Chat {
            id,
            group_id: GroupId::from_slice(&[0; 32]),
            last_read: Utc::now(),
            status: ChatStatus::Active,
            conversation_type: ChatType::Group,
            attributes: ChatAttributes {
                title: "Test conversation".to_string(),
                picture: None,
            },
        }
    }

    #[sqlx::test]
    async fn store_load(mut connection: PoolConnection<Sqlite>) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_chat();
        conversation
            .store(&mut connection, &mut store_notifier)
            .await?;
        let loaded = Chat::load(&mut connection, &conversation.id)
            .await?
            .expect("missing conversation");
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[sqlx::test]
    async fn store_load_by_group_id(mut connection: PoolConnection<Sqlite>) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_chat();
        conversation
            .store(&mut connection, &mut store_notifier)
            .await?;
        let loaded = Chat::load_by_group_id(&mut connection, &conversation.group_id)
            .await?
            .expect("missing conversation");
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[sqlx::test]
    async fn store_load_all(mut connection: PoolConnection<Sqlite>) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation_a = test_chat();
        conversation_a
            .store(&mut connection, &mut store_notifier)
            .await?;

        let conversation_b = test_chat();
        conversation_b
            .store(&mut connection, &mut store_notifier)
            .await?;

        let loaded = Chat::load_all(&mut connection).await?;
        assert_eq!(loaded, [conversation_a, conversation_b]);

        Ok(())
    }

    #[sqlx::test]
    async fn update_conversation_picture(
        mut connection: PoolConnection<Sqlite>,
    ) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let mut conversation = test_chat();
        conversation
            .store(&mut connection, &mut store_notifier)
            .await?;

        let new_picture = [1, 2, 3];
        Chat::update_picture(
            &mut *connection,
            &mut store_notifier,
            conversation.id,
            Some(&new_picture),
        )
        .await?;

        conversation.attributes.picture = Some(new_picture.to_vec());

        let loaded = Chat::load(&mut connection, &conversation.id)
            .await?
            .unwrap();
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[sqlx::test]
    async fn update_conversation_status(
        mut connection: PoolConnection<Sqlite>,
    ) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let mut conversation = test_chat();
        conversation
            .store(&mut connection, &mut store_notifier)
            .await?;

        let mut past_members = vec![
            UserId::random("localhost".parse().unwrap()),
            UserId::random("localhost".parse().unwrap()),
        ];
        // implicit assumption: past members are sorted
        past_members.sort_unstable();

        let status = ChatStatus::Inactive(InactiveChat::new(past_members));
        Chat::update_status(
            &mut connection,
            &mut store_notifier,
            conversation.id,
            &status,
        )
        .await?;

        conversation.status = status;
        let loaded = Chat::load(&mut connection, &conversation.id)
            .await?
            .unwrap();
        assert_eq!(loaded, conversation);

        Ok(())
    }

    #[sqlx::test]
    async fn delete(mut connection: PoolConnection<Sqlite>) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation = test_chat();
        conversation
            .store(&mut connection, &mut store_notifier)
            .await?;
        let loaded = Chat::load(&mut connection, &conversation.id)
            .await?
            .unwrap();
        assert_eq!(loaded, conversation);

        Chat::delete(&mut *connection, &mut store_notifier, conversation.id).await?;
        let loaded = Chat::load(&mut connection, &conversation.id).await?;
        assert!(loaded.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn counters(mut connection: PoolConnection<Sqlite>) -> anyhow::Result<()> {
        let mut store_notifier = StoreNotifier::noop();

        let conversation_a = test_chat();
        conversation_a
            .store(&mut connection, &mut store_notifier)
            .await?;

        let conversation_b = test_chat();
        conversation_b
            .store(&mut connection, &mut store_notifier)
            .await?;

        let message_a = test_conversation_message(conversation_a.id());
        let message_b = test_conversation_message(conversation_b.id());

        message_a
            .store(&mut *connection, &mut store_notifier)
            .await?;
        message_b
            .store(&mut *connection, &mut store_notifier)
            .await?;

        let n = Chat::messages_count(&mut *connection, conversation_a.id()).await?;
        assert_eq!(n, 1);

        let n = Chat::messages_count(&mut *connection, conversation_b.id()).await?;
        assert_eq!(n, 1);

        let n = Chat::global_unread_message_count(&mut *connection).await?;
        assert_eq!(n, 2);

        let mut txn = connection.begin().await?;
        Chat::mark_as_read(
            &mut txn,
            &mut store_notifier,
            [(
                conversation_a.id(),
                message_a.timestamp() - Duration::seconds(1),
            )],
        )
        .await?;
        txn.commit().await?;
        let n = Chat::unread_messages_count(&mut *connection, conversation_a.id()).await?;
        assert_eq!(n, 1);

        let mut txn = connection.begin().await?;
        Chat::mark_as_read(
            &mut txn,
            &mut store_notifier,
            [(conversation_a.id(), Utc::now())],
        )
        .await?;
        txn.commit().await?;
        let n = Chat::unread_messages_count(&mut *connection, conversation_a.id()).await?;
        assert_eq!(n, 0);

        let mut txn = connection.begin().await?;
        Chat::mark_as_read_until_message_id(
            &mut txn,
            &mut store_notifier,
            conversation_b.id(),
            MessageId::random(),
            &UserId::random("localhost".parse().unwrap()),
        )
        .await?;
        txn.commit().await?;
        let n = Chat::unread_messages_count(&mut *connection, conversation_b.id()).await?;
        assert_eq!(n, 1);

        let mut txn = connection.begin().await?;
        Chat::mark_as_read_until_message_id(
            &mut txn,
            &mut store_notifier,
            conversation_b.id(),
            message_b.id(),
            &UserId::random("localhost".parse().unwrap()),
        )
        .await?;
        txn.commit().await?;
        let n = Chat::unread_messages_count(&mut *connection, conversation_b.id()).await?;
        assert_eq!(n, 0);

        let n = Chat::global_unread_message_count(&mut *connection).await?;
        assert_eq!(n, 0);

        Ok(())
    }
}
