// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Data migrations implemented in Rust that cannot be expressed in SQL.

use mimi_content::content_container::MimiContentV1;
use phnxcommon::codec::{BlobDecoded, BlobEncoded, PhnxCodec};
use sqlx::{SqlitePool, migrate::Migrate, query, query_as};
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::{ConversationMessageId, conversations::messages::persistence::VersionedMessage};

const MESSAGE_STATUS_MIGRATION_VERSION: i64 = 20250703133517;

/// Migrate data in the database that cannot be expressed in SQL.
pub(crate) async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let migrations = pool.acquire().await?.list_applied_migrations().await?;
    let has_message_status = migrations
        .iter()
        .any(|m| m.version == MESSAGE_STATUS_MIGRATION_VERSION);
    if !has_message_status {
        if let Err(error) = convert_messages_v1_to_v2(pool).await {
            error!(%error, "Failed to convert messages from version 1 to version 2");
        }
    }
    Ok(())
}

/// Convert all versioned v1 messages to v2 messages in the database.
async fn convert_messages_v1_to_v2(pool: &SqlitePool) -> anyhow::Result<()> {
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
        let content_v1: MimiContentV1 = PhnxCodec::from_slice(&content)?;
        let content_v2 = BlobEncoded(content_v1.upgrade());
        query!(
            "UPDATE conversation_messages SET content = ? WHERE message_id = ?",
            content_v2,
            message_id,
        )
        .execute(&mut *write_connection)
        .await?;

        num_messages += 1;
    }

    if num_messages > 0 {
        info!(
            num_messages,
            "Converted messages from version 1 to version 2",
        );
    }

    Ok(())
}
