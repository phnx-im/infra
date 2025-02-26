// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use openmls::group::GroupId;
use rusqlite::{Connection, OptionalExtension, Transaction, named_params, params};
use tracing::info;

use crate::{
    Conversation, ConversationAttributes, ConversationId, ConversationMessageId,
    ConversationStatus, ConversationType,
    store::StoreNotifier,
    utils::persistence::{GroupIdRefWrapper, GroupIdWrapper, Storable},
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

impl Conversation {
    pub(crate) fn store(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
    ) -> Result<(), rusqlite::Error> {
        info!(
            id =% self.id,
            title =% self.attributes().title(),
            "Storing conversation"
        );
        let group_id = GroupIdRefWrapper::from(&self.group_id);
        connection.execute(
            "INSERT INTO conversations (conversation_id,
                conversation_title,
                conversation_picture,
                group_id,
                last_read,
                conversation_status,
                conversation_type)
            VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                self.id,
                self.attributes().title(),
                self.attributes().picture(),
                group_id,
                self.last_read,
                self.status(),
                self.conversation_type(),
            ],
        )?;
        notifier.add(self.id);
        Ok(())
    }

    pub(crate) fn load(
        connection: &Connection,
        conversation_id: &ConversationId,
    ) -> Result<Option<Conversation>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT
                conversation_id,
                conversation_title,
                conversation_picture,
                group_id,
                last_read,
                conversation_status,
                conversation_type
            FROM conversations
            WHERE conversation_id = ?",
        )?;
        stmt.query_row(params![conversation_id], Self::from_row)
            .optional()
    }

    pub(crate) fn load_by_group_id(
        connection: &Connection,
        group_id: &GroupId,
    ) -> Result<Option<Conversation>, rusqlite::Error> {
        let group_id = GroupIdRefWrapper::from(group_id);
        let mut stmt = connection.prepare(
            "SELECT
                conversation_id,
                conversation_title,
                conversation_picture,
                group_id,
                last_read,
                conversation_status,
                conversation_type
            FROM conversations
            WHERE group_id = ?",
        )?;
        stmt.query_row(params![group_id], Self::from_row).optional()
    }

    pub(crate) fn load_all(connection: &Connection) -> Result<Vec<Conversation>, rusqlite::Error> {
        let mut stmt = connection.prepare(
            "SELECT
                conversation_id,
                conversation_title,
                conversation_picture,
                group_id,
                last_read,
                conversation_status,
                conversation_type
            FROM conversations",
        )?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }

    pub(super) fn update_picture(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        conversation_picture: Option<&[u8]>,
    ) -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE conversations SET conversation_picture = ? WHERE conversation_id = ?",
            params![conversation_picture, conversation_id],
        )?;
        notifier.update(conversation_id);
        Ok(())
    }

    pub(super) fn update_status(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        status: &ConversationStatus,
    ) -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE conversations SET conversation_status = ? WHERE conversation_id = ?",
            params![status, conversation_id],
        )?;
        notifier.update(conversation_id);
        Ok(())
    }

    pub(crate) fn delete(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "DELETE FROM conversations WHERE conversation_id = ?",
            params![conversation_id],
        )?;
        notifier.remove(conversation_id);
        Ok(())
    }

    /// Set the `last_read` marker of all conversations with the given
    /// [`ConversationId`]s to the given timestamps. This is used to mark all
    /// messages up to this timestamp as read.
    pub(crate) fn mark_as_read(
        transaction: &Transaction,
        notifier: &mut StoreNotifier,
        mark_as_read_data: impl IntoIterator<Item = (ConversationId, DateTime<Utc>)>,
    ) -> Result<(), rusqlite::Error> {
        let mut unread_messages_stmt = transaction.prepare(
            "SELECT message_id from conversation_messages
            INNER JOIN conversations c ON c.conversation_id = :conversation_id
            WHERE c.conversation_id = :conversation_id AND timestamp > c.last_read",
        )?;
        let mut update_stmt = transaction.prepare(
            "UPDATE conversations
                SET last_read = :timestamp
                WHERE conversation_id = :conversation_id AND last_read < :timestamp",
        )?;
        for (conversation_id, timestamp) in mark_as_read_data {
            let unread_messages: Result<Vec<ConversationMessageId>, _> = unread_messages_stmt
                .query_map(
                    named_params! {
                        ":conversation_id": conversation_id,
                    },
                    |row| row.get(0),
                )?
                .collect();
            for message_id in unread_messages? {
                notifier.update(message_id);
            }
            let updated = update_stmt.execute(named_params! {
                ":timestamp": timestamp,
                ":conversation_id": conversation_id,
            })?;
            if updated == 1 {
                notifier.update(conversation_id);
            }
        }
        Ok(())
    }

    /// Mark all messages in the conversation as read until including the given message id.
    pub(crate) fn mark_as_read_until_message_id(
        connection: &Connection,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        until_message_id: ConversationMessageId,
    ) -> rusqlite::Result<bool> {
        let timestamp: Option<DateTime<Utc>> = connection
            .query_row(
                "SELECT timestamp FROM conversation_messages WHERE message_id = ?",
                params![until_message_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(timestamp) = timestamp else {
            return Ok(false);
        };
        let updated = connection.execute(
            "UPDATE conversations SET last_read = :timestamp
            WHERE conversation_id = :conversation_id AND last_read != :timestamp",
            named_params! {
                ":conversation_id": conversation_id,
                ":timestamp": timestamp,
            },
        )?;
        let marked_as_read = updated == 1;
        if marked_as_read {
            notifier.update(conversation_id);
        }
        Ok(marked_as_read)
    }

    pub(crate) fn global_unread_message_count(
        connection: &Connection,
    ) -> Result<usize, rusqlite::Error> {
        connection.query_row(
            "SELECT
                COUNT(cm.conversation_id) AS total_unread_messages
            FROM
                conversations c
            LEFT JOIN
                conversation_messages cm
            ON
                c.conversation_id = cm.conversation_id
                AND cm.sender != 'system'
                AND cm.timestamp > c.last_read;",
            [],
            |row| row.get(0),
        )
    }

    pub(crate) fn messages_count(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<usize, rusqlite::Error> {
        connection.query_row(
            "SELECT
                COUNT(*)
            FROM
                conversation_messages cm
            WHERE
                cm.conversation_id = :conversation_id
                AND cm.sender != 'system';",
            named_params! {":conversation_id": conversation_id},
            |row| row.get(0),
        )
    }

    pub(crate) fn unread_messages_count(
        connection: &Connection,
        conversation_id: ConversationId,
    ) -> Result<usize, rusqlite::Error> {
        connection.query_row(
            "SELECT
                    COUNT(*)
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
                    )",
            named_params! {":conversation_id": conversation_id},
            |row| row.get(0),
        )
    }

    pub(super) fn set_conversation_type(
        &self,
        connection: &Connection,
        notifier: &mut StoreNotifier,
        conversation_type: &ConversationType,
    ) -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE conversations SET conversation_type = ? WHERE conversation_id = ?",
            params![conversation_type, self.id],
        )?;
        notifier.update(self.id);
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use chrono::Duration;
    use uuid::Uuid;

    use crate::{
        InactiveConversation,
        conversations::messages::persistence::tests::{test_connection, test_conversation_message},
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
