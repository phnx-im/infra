// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use phnxcommon::identifiers::AttachmentId;
use sqlx::{
    Database, Decode, Encode, Sqlite, SqliteExecutor, Type, encode::IsNull, error::BoxDynError,
    query,
};

use crate::{ConversationId, ConversationMessageId, store::StoreNotifier};

/// A record of an attachment.
///
/// Content is intentially not included in this struct.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(crate) struct AttachmentRecord {
    pub(super) attachment_id: AttachmentId,
    pub(super) conversation_id: ConversationId,
    pub(super) conversation_message_id: ConversationMessageId,
    pub(super) content_type: String,
    pub(super) status: AttachmentStatus,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq, Eq))]
#[repr(u8)]
pub enum AttachmentStatus {
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

impl AttachmentStatus {
    fn from_u32(value: u32) -> Self {
        match value {
            1 => Self::Pending,
            2 => Self::Downloading,
            3 => Self::Ready,
            4 => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

pub enum AttachmentContent {
    /// There no such attachment
    None,
    /// Fully downloaded
    Ready(Vec<u8>),
    /// Not yet started to download
    Pending,
    /// Currently downloading
    Downloading,
    /// Failed to download
    Failed,
    /// Unknown status
    Unknown,
}

impl AttachmentContent {
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            AttachmentContent::Ready(content) => Some(content),
            _ => None,
        }
    }
}

impl Type<Sqlite> for AttachmentStatus {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        // Note: don't use u8, sqlx gets confused with:
        // ```
        // mismatched types; Rust type (as SQL type `INTEGER`) is not compatible with SQL type `INTEGER`
        // ```
        <u32 as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for AttachmentStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode(*self as u32, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for AttachmentStatus {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let idx: u32 = Decode::<Sqlite>::decode(value)?;
        Ok(Self::from_u32(idx))
    }
}

impl AttachmentRecord {
    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        content: Option<&[u8]>,
    ) -> sqlx::Result<()> {
        query!(
            "INSERT INTO attachments (
                attachment_id,
                conversation_id,
                conversation_message_id,
                content_type,
                content,
                status,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            self.attachment_id,
            self.conversation_id,
            self.conversation_message_id,
            self.content_type,
            content,
            self.status,
            self.created_at,
        )
        .execute(executor)
        .await?;
        notifier.add(self.attachment_id);
        Ok(())
    }
}
