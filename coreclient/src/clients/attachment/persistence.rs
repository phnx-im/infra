use sqlx::SqliteExecutor;
use uuid::Uuid;

use crate::ConversationId;

pub(super) struct AttachmentRecord {
    pub(super) attachment_id: Uuid,
    pub(super) conversation_id: ConversationId,
    pub(super) content_type: String,
    pub(super) content: Vec<u8>,
}

pub(super) struct AttachmentImageRecord {
    pub(super) attachment_id: Uuid,
    pub(super) thumbnail: Vec<u8>,
    pub(super) thumbnail_size: u32,
    pub(super) blurhash: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl AttachmentRecord {
    pub(super) async fn store(&self, executor: impl SqliteExecutor<'_>) -> anyhow::Result<()> {
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
            self.content,
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
                    thumbnail,
                    thumbnail_size,
                    blurhash,
                    width,
                    height
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#,
            self.attachment_id,
            self.thumbnail,
            self.thumbnail_size,
            self.blurhash,
            self.width,
            self.height,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}
