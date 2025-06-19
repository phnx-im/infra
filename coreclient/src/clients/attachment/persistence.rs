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
    pub(super) thumbnail_id: Uuid,
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
    pub(super) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        thumbnail_content: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let thumbnail_content = thumbnail_content.as_ref();
        sqlx::query!(
            r#"
                INSERT INTO attachment_images (
                    attachment_id,
                    thumbnail_id,
                    thumbnail_content,
                    blurhash,
                    width,
                    height
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
            self.attachment_id,
            self.thumbnail_id,
            thumbnail_content,
            self.blurhash,
            self.width,
            self.height,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}
