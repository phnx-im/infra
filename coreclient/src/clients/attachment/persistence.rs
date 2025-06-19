// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::SqliteExecutor;
use uuid::Uuid;

use crate::ConversationId;

/// A record of an attachment.
///
/// Content is intentially not included in this struct.
pub(super) struct AttachmentRecord {
    pub(super) attachment_id: Uuid,
    pub(super) conversation_id: ConversationId,
    pub(super) content_type: String,
}

/// Additional information about an attachment when it is an image.
///
/// Thumbnail content is intentially not included in this struct.
pub(super) struct AttachmentImageRecord {
    pub(super) attachment_id: Uuid,
    pub(super) blurhash: String,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl AttachmentRecord {
    pub(super) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        content: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let content = content.as_ref();
        sqlx::query!(
            r#"
                INSERT INTO attachments (
                    attachment_id,
                    conversation_id,
                    content_type,
                    content
                ) VALUES (?, ?, ?, ?)
                "#,
            self.attachment_id,
            self.conversation_id,
            self.content_type,
            content,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

impl AttachmentImageRecord {
    pub(super) async fn store(&self, executor: impl SqliteExecutor<'_>) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO attachment_images (
                    attachment_id,
                    blurhash,
                    width,
                    height
                ) VALUES (?, ?, ?, ?)
                "#,
            self.attachment_id,
            self.blurhash,
            self.width,
            self.height,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}
