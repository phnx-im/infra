// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Data migrations implemented in Rust that cannot be expressed in SQL.

use aircommon::codec::{AirCodec, BlobDecoded, BlobEncoded};
use mimi_content::content_container::MimiContentV1;
use sqlx::{SqlitePool, migrate::Migrate, query, query_as};
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::{ConversationMessageId, conversations::messages::persistence::VersionedMessage};

const MESSAGE_STATUS_MIGRATION_VERSION: i64 = 20250703133517;

/// Migrate data in the database that cannot be expressed in SQL.
pub(crate) async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let Some(migrations) = pool.acquire().await?.list_applied_migrations().await.ok() else {
        // The migrations might not yet exist
        return Ok(());
    };

    let has_message_status = migrations
        .iter()
        .any(|m| m.version == MESSAGE_STATUS_MIGRATION_VERSION);
    if !has_message_status && let Err(error) = convert_messages_v1_to_v2(pool).await {
        error!(%error, "Failed to convert messages from version 1 to version 2");
    }

    Ok(())
}

/// Convert all versioned v1 messages to v2 messages in the database.
async fn convert_messages_v1_to_v2(pool: &SqlitePool) -> anyhow::Result<usize> {
    info!("Data migration: Converting messages from version 1 to version 2");

    let mut write_connection = pool.acquire().await?;
    let mut read_connection = pool.acquire().await?;

    struct Record {
        message_id: ConversationMessageId,
        content: BlobDecoded<VersionedMessage>,
    }

    let mut records = query_as!(
        Record,
        r#"SELECT
            message_id AS "message_id: _",
            content AS "content: _"
        FROM conversation_messages"#
    )
    .fetch(read_connection.as_mut());

    let mut num_messages = 0;

    while let Some(Record {
        message_id,
        content: BlobDecoded(VersionedMessage { version, content }),
    }) = records.next().await.transpose()?
        && version == 1
    {
        let content_v1: MimiContentV1 = AirCodec::from_slice(&content)?;
        let content_v2 = content_v1.upgrade();
        let Ok(message) = VersionedMessage::from_mimi_content(&content_v2) else {
            error!(
                ?message_id,
                "Failed to convert message from version 1 to version 2; skip"
            );
            continue;
        };
        let message = BlobEncoded(message);
        query!(
            "UPDATE conversation_messages SET content = ? WHERE message_id = ?",
            message,
            message_id,
        )
        .execute(&mut *write_connection)
        .await?;

        num_messages += 1;
    }

    info!(
        num_messages,
        "Converted messages from version 1 to version 2",
    );

    Ok(num_messages)
}

#[cfg(test)]
mod test {
    use aircommon::{assert_matches, identifiers::UserId, time::TimeStamp};
    use sqlx::{SqliteConnection, migrate::Migrate};

    use crate::{
        ConversationId,
        conversations::{
            messages::persistence::tests::test_conversation_message,
            persistence::tests::test_conversation,
        },
        store::StoreNotifier,
    };

    use super::*;

    async fn store_raw_test_message_v1(
        connection: &mut SqliteConnection,
        converation_id: ConversationId,
    ) -> anyhow::Result<()> {
        let message_id = ConversationMessageId::random();
        let user_id = UserId::random("localhost".parse().unwrap());
        let user_uuid = user_id.uuid();
        let user_domain = user_id.domain();
        let timestamp = TimeStamp::now();
        let mimi_content = MimiContentV1 {
            topic_id: vec![1; 32].into(),
            ..Default::default()
        };
        let mimi_content_bytes = AirCodec::to_vec(&mimi_content).unwrap();
        let content = VersionedMessage {
            version: 1,
            content: mimi_content_bytes,
        };

        sqlx::query(
            "INSERT INTO conversation_messages (
                message_id,
                conversation_id,
                sender_user_uuid,
                sender_user_domain,
                timestamp,
                content,
                sent
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(message_id)
        .bind(converation_id)
        .bind(user_uuid)
        .bind(user_domain)
        .bind(timestamp)
        .bind(BlobEncoded(content))
        .bind(false)
        .execute(connection)
        .await?;

        Ok(())
    }

    #[sqlx::test(migrations = false)]
    async fn convert_messages_v1_to_v2_some_messages(pool: SqlitePool) -> anyhow::Result<()> {
        let mut connection = pool.acquire().await?;
        connection.ensure_migrations_table().await?;
        let migrator = sqlx::migrate!();

        let itx = migrator
            .iter()
            .position(|m| m.version == MESSAGE_STATUS_MIGRATION_VERSION)
            .unwrap();

        // apply migrations up to the message status migration
        for migration in migrator.iter().take(itx) {
            connection.apply(migration).await?;
        }

        let mut notifier = StoreNotifier::noop();
        let conversation = test_conversation();
        conversation.store(&mut connection, &mut notifier).await?;

        // Store v1 messages
        store_raw_test_message_v1(&mut connection, conversation.id()).await?;
        store_raw_test_message_v1(&mut connection, conversation.id()).await?;
        store_raw_test_message_v1(&mut connection, conversation.id()).await?;

        // finish migration
        for migration in migrator.iter().skip(itx) {
            connection.apply(migration).await?;
        }

        // Store v2 messages
        test_conversation_message(conversation.id())
            .store(connection.as_mut(), &mut notifier)
            .await?;
        test_conversation_message(conversation.id())
            .store(connection.as_mut(), &mut notifier)
            .await?;
        test_conversation_message(conversation.id())
            .store(connection.as_mut(), &mut notifier)
            .await?;

        assert_matches!(convert_messages_v1_to_v2(&pool).await, Ok(3));

        Ok(())
    }
}
