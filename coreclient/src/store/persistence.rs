// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::QualifiedUserName;
use rusqlite::{
    params,
    types::{FromSql, ToSqlOutput, Value},
    Connection, ToSql,
};
use tracing::error;

use super::{notification::StoreEntityKind, StoreEntityId, StoreNotification, StoreOperation};

impl ToSql for StoreEntityId {
    /// Lossy conversion to SQLite value: the type is not stored.
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            StoreEntityId::User(user_name) => user_name.to_sql(),
            StoreEntityId::Conversation(conversation_id) => conversation_id.to_sql(),
            StoreEntityId::Message(message_id) => message_id.to_sql(),
        }
    }
}

impl ToSql for StoreEntityKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(*self as i64)))
    }
}

impl FromSql for StoreEntityKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_i64() {
            Ok(0) => Ok(StoreEntityKind::User),
            Ok(1) => Ok(StoreEntityKind::Conversation),
            Ok(2) => Ok(StoreEntityKind::Message),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl StoreNotification {
    pub const CREATE_TABLE_STATEMENT: &str = "
        CREATE TABLE IF NOT EXISTS store_notifications (
            entity_id BLOB NOT NULL,
            kind INTEGER NOT NULL,
            added BOOLEAN NOT NULL,
            updated BOOLEAN NOT NULL,
            removed BOOLEAN NOT NULL,
            PRIMARY KEY (entity_id, kind)
        );";

    pub(crate) fn enqueue(&self, connection: &mut Connection) -> Result<(), rusqlite::Error> {
        let transaction = connection.transaction()?;
        let mut statement = transaction.prepare(
            "INSERT INTO store_notifications (entity_id, kind, added, updated, removed)
            VALUES (?, ?, ?, ?, ?)",
        )?;
        for (entity_id, operation) in &self.ops {
            let kind = entity_id.kind();
            statement.execute(params![
                entity_id,
                kind,
                operation == &StoreOperation::Add,
                operation == &StoreOperation::Update,
                operation == &StoreOperation::Remove,
            ])?;
        }
        drop(statement);
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn dequeue(
        connection: &mut Connection,
    ) -> Result<StoreNotification, rusqlite::Error> {
        let mut statement = connection.prepare("DELETE FROM store_notifications RETURNING *")?;
        let ops = statement
            .query_map(params![], |row| {
                dbg!(&row);

                let kind: StoreEntityKind = row.get(1)?;
                let entity_id = match kind {
                    StoreEntityKind::User => {
                        let user_name: QualifiedUserName = row.get(0)?;
                        StoreEntityId::User(user_name)
                    }
                    StoreEntityKind::Conversation => {
                        let id: crate::ConversationId = row.get(0)?;
                        StoreEntityId::Conversation(id)
                    }
                    StoreEntityKind::Message => {
                        let id: crate::ConversationMessageId = row.get(0)?;
                        StoreEntityId::Message(id)
                    }
                };

                // TODO: Precendence of operations will be removed when we switch to a bitset.
                let added = row.get(2)?;
                let updated = row.get(3)?;
                let removed = row.get(4)?;
                let op = if added {
                    StoreOperation::Add
                } else if updated {
                    StoreOperation::Update
                } else if removed {
                    StoreOperation::Remove
                } else {
                    error!(?entity_id, "Invalid store notification; missing operation");
                    return Ok(None);
                };

                Ok(Some((entity_id, op)))
            })?
            .filter_map(|res| res.transpose())
            .collect::<rusqlite::Result<_>>()?;
        Ok(StoreNotification { ops })
    }
}

#[cfg(test)]
mod tests {
    use crate::{ConversationId, ConversationMessageId};

    use super::*;

    #[test]
    fn queue_dequeue_notification() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(StoreNotification::CREATE_TABLE_STATEMENT)
            .unwrap();

        let mut notification = StoreNotification::default();
        notification.ops.insert(
            StoreEntityId::User("alice@localhost".parse().unwrap()),
            StoreOperation::Add,
        );
        notification.ops.insert(
            StoreEntityId::Conversation(ConversationId {
                uuid: uuid::Uuid::new_v4(),
            }),
            StoreOperation::Update,
        );
        notification.ops.insert(
            StoreEntityId::Message(ConversationMessageId {
                uuid: uuid::Uuid::new_v4(),
            }),
            StoreOperation::Remove,
        );

        notification.enqueue(&mut connection).unwrap();

        let dequeued_notification = StoreNotification::dequeue(&mut connection).unwrap();
        assert_eq!(notification, dequeued_notification);

        let dequeued_notification = StoreNotification::dequeue(&mut connection).unwrap();
        assert!(dequeued_notification.is_empty());
    }
}
