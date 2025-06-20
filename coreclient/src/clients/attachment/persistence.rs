// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::{
    Database, Decode, Encode, Sqlite, SqliteExecutor, Type, encode::IsNull, error::BoxDynError,
};
use uuid::Uuid;

use crate::{AttachmentId, ConversationId, store::StoreNotifier};

impl Type<Sqlite> for AttachmentId {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Uuid as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for AttachmentId {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(&self.uuid, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for AttachmentId {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let id: Uuid = Decode::<Sqlite>::decode(value)?;
        Ok(Self::new(id))
    }
}

/// A record of an attachment.
///
/// Content is intentially not included in this struct.
pub(super) struct AttachmentRecord {
    pub(super) attachment_id: AttachmentId,
    pub(super) conversation_id: ConversationId,
    pub(super) content_type: String,
    pub(super) status: AttachmentStatus,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum AttachmentStatus {
    /// Unknown status
    Unknown = 0,
    /// The download has not started yet.
    Pending = 1,
    /// The download is in progress.
    Downloading = 2,
    /// The download has completed successfully.
    Ready = 3,
    /// The download has failed.
    Failed = 4,
}

impl Type<Sqlite> for AttachmentStatus {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <u8 as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for AttachmentStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(*self as u8, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for AttachmentStatus {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let idx: u8 = Decode::<Sqlite>::decode(value)?;
        Ok(Self::from(idx))
    }
}

impl From<u8> for AttachmentStatus {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Pending,
            2 => Self::Downloading,
            3 => Self::Ready,
            4 => Self::Failed,
            _ => Self::Unknown,
        }
    }
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
        notifier: &mut StoreNotifier,
        content: Option<&[u8]>,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO attachments (
                    attachment_id,
                    conversation_id,
                    content_type,
                    content,
                    status
                ) VALUES (?, ?, ?, ?, ?)
                "#,
            self.attachment_id,
            self.conversation_id,
            self.content_type,
            content,
            self.status,
        )
        .execute(executor)
        .await?;
        notifier.add(self.attachment_id);
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
