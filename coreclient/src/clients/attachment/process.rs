// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Process incoming attachments.

use anyhow::{Context, ensure};
use mimi_content::content_container::{NestedPart, NestedPartContent};
use phnxcommon::time::TimeStamp;
use sqlx::SqliteTransaction;
use tracing::error;

use crate::{
    AttachmentId,
    clients::{
        CoreUser,
        attachment::{AttachmentRecord, persistence::AttachmentStatus},
    },
    conversations::{ConversationId, messages::TimestampedMessage},
    store::StoreNotifier,
};

const MAX_RECURSION_DEPTH: usize = 3;

impl CoreUser {
    /// Collect attachments from messages and store them in the store as pending.
    pub(crate) async fn handle_attachments(
        &self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        messages: &[TimestampedMessage],
    ) {
        for message in messages {
            if let Err(error) = self
                .handle_attachment(txn, notifier, conversation_id, message)
                .await
            {
                error!(%error, "Failed to process attachment");
            }
        }
    }

    async fn handle_attachment(
        &self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        message: &TimestampedMessage,
    ) -> anyhow::Result<()> {
        let Some(content) = message.mimi_content() else {
            return Ok(());
        };
        self.handle_nested_part(
            txn,
            notifier,
            conversation_id,
            message.timestamp(),
            &content.nested_part,
            0,
        )
        .await
    }

    async fn handle_nested_part(
        &self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        conversation_id: ConversationId,
        timestamp: TimeStamp,
        nested_part: &NestedPart,
        recursion_depth: usize,
    ) -> anyhow::Result<()> {
        ensure!(
            recursion_depth < MAX_RECURSION_DEPTH,
            "Failed to handle attachment due to maximum recursion depth reached"
        );

        match &nested_part.part {
            NestedPartContent::ExternalPart {
                url, content_type, ..
            } => {
                let attachment_id = AttachmentId::from_url(url)
                    .with_context(|| format!("invalid attachment url: {url}"))?;

                AttachmentRecord {
                    attachment_id,
                    conversation_id,
                    content_type: content_type.to_owned(),
                    status: AttachmentStatus::Pending,
                    created_at: timestamp.into(),
                }
                .store(txn.as_mut(), notifier, None)
                .await?;
            }
            NestedPartContent::MultiPart { parts, .. } => {
                for part in parts {
                    if let Err(error) = Box::pin(self.handle_nested_part(
                        txn,
                        notifier,
                        conversation_id,
                        timestamp,
                        part,
                        recursion_depth + 1,
                    ))
                    .await
                    {
                        error!(%error, "Failed to process attachment nested part");
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
}
