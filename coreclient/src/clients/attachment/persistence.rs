// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};
use mimi_content::content_container::{EncryptionAlgorithm, HashAlgorithm};
use phnxcommon::identifiers::AttachmentId;
use sqlx::{
    Database, Decode, Encode, Sqlite, SqliteExecutor, Type, encode::IsNull, error::BoxDynError,
    query, query_as, query_scalar,
};
use uuid::Uuid;

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
    pub(super) arrived_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq, Eq))]
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
                arrived_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            self.attachment_id,
            self.conversation_id,
            self.conversation_message_id,
            self.content_type,
            content,
            self.status,
            self.arrived_at,
        )
        .execute(executor)
        .await?;
        notifier.add(self.attachment_id);
        Ok(())
    }

    pub(crate) async fn load_all_pending(
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<Vec<AttachmentId>> {
        query_scalar!(
            r#"SELECT
                attachment_id AS "attachment_id: AttachmentId"
            FROM attachments
            WHERE status = ?
            ORDER BY arrived_at ASC"#,
            AttachmentStatus::Pending
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn delete(
        executor: impl SqliteExecutor<'_>,
        attachment_id: AttachmentId,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM attachments WHERE attachment_id = ?",
            attachment_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        attachment_id: AttachmentId,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            AttachmentRecord,
            r#"
                SELECT
                    attachment_id AS "attachment_id: _",
                    conversation_id AS "conversation_id: _",
                    conversation_message_id AS "conversation_message_id: _",
                    content_type AS "content_type: _",
                    status AS "status: _",
                    arrived_at AS "arrived_at: _"
                FROM attachments
                WHERE attachment_id = ?"#,
            attachment_id
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn update_status(
        executor: impl SqliteExecutor<'_>,
        attachment_id: AttachmentId,
        status: AttachmentStatus,
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE attachments SET status = ? WHERE attachment_id = ?",
            status,
            attachment_id,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_as_ready(
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
        attachment_id: AttachmentId,
        content: &[u8],
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE attachments SET status = ?, content = ? WHERE attachment_id = ?",
            AttachmentStatus::Ready,
            content,
            attachment_id,
        )
        .execute(executor)
        .await?;
        notifier.update(attachment_id);
        Ok(())
    }
}

impl AttachmentImageRecord {
    pub(super) async fn store(&self, executor: impl SqliteExecutor<'_>) -> anyhow::Result<()> {
        query!(
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

pub(crate) struct PendingAttachmentRecord {
    pub(super) attachment_id: AttachmentId,
    pub(super) size: u64,
    pub(super) enc_alg: EncryptionAlgorithm,
    pub(super) enc_key: Vec<u8>,
    pub(super) nonce: Vec<u8>,
    pub(super) aad: Vec<u8>,
    pub(super) hash_alg: HashAlgorithm,
    pub(super) hash: Vec<u8>,
}

impl PendingAttachmentRecord {
    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let size = self.size as i64;
        let enc_alg = self.enc_alg.as_u16() as i64;
        let hash_alg = self.hash_alg.as_u8() as i64;
        query!(
            "INSERT INTO pending_attachments (
                attachment_id,
                size,
                enc_alg,
                enc_key,
                nonce,
                aad,
                hash_alg,
                hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            self.attachment_id,
            size,
            enc_alg,
            self.enc_key,
            self.nonce,
            self.aad,
            hash_alg,
            self.hash,
        )
        .execute(executor)
        .await?;
        notifier.add(self.attachment_id);
        Ok(())
    }

    pub(crate) async fn load_pending(
        executor: impl SqliteExecutor<'_>,
        attachment_id: AttachmentId,
    ) -> sqlx::Result<Option<Self>> {
        struct SqlPendingAttachmentRecord {
            size: u64,
            enc_alg: u16,
            enc_key: Vec<u8>,
            nonce: Vec<u8>,
            aad: Vec<u8>,
            hash_alg: u8,
            hash: Vec<u8>,
        }

        let record = query_as!(
            SqlPendingAttachmentRecord,
            r#"
                SELECT
                    pa.size AS "size: _",
                    pa.enc_alg AS "enc_alg: _",
                    pa.enc_key AS "enc_key: _",
                    pa.nonce AS "nonce: _",
                    pa.aad AS "aad: _",
                    pa.hash_alg AS "hash_alg: _",
                    pa.hash AS "hash: _"
                FROM pending_attachments pa
                INNER JOIN attachments a ON a.attachment_id = pa.attachment_id
                WHERE pa.attachment_id = ? AND a.status = 1
            "#,
            attachment_id
        )
        .fetch_optional(executor)
        .await?;
        Ok(record.map(
            |SqlPendingAttachmentRecord {
                 size,
                 enc_alg,
                 enc_key,
                 nonce,
                 aad,
                 hash_alg,
                 hash,
             }| {
                PendingAttachmentRecord {
                    attachment_id,
                    size,
                    enc_alg: EncryptionAlgorithm::from_u16(enc_alg),
                    enc_key,
                    nonce,
                    aad,
                    hash_alg: HashAlgorithm::from_u8(hash_alg),
                    hash,
                }
            },
        ))
    }

    pub(crate) async fn delete(
        executor: impl SqliteExecutor<'_>,
        attachment_id: AttachmentId,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM pending_attachments WHERE attachment_id = ?",
            attachment_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use chrono::SubsecRound;
    use sqlx::Pool;

    use crate::conversations::{
        messages::persistence::tests::test_conversation_message,
        persistence::tests::test_conversation,
    };

    use super::*;

    #[sqlx::test]
    async fn attachment_record_store_and_load(pool: Pool<Sqlite>) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut notifier)
            .await?;
        let message = test_conversation_message(conversation.id());
        message.store(&pool, &mut notifier).await?;

        let attachment_id = AttachmentId::new(Uuid::new_v4());
        let arrived_at = Utc::now().round_subsecs(6);

        let record = AttachmentRecord {
            attachment_id,
            conversation_id: conversation.id(),
            conversation_message_id: message.id(),
            content_type: "image/png".to_string(),
            status: AttachmentStatus::Pending,
            arrived_at,
        };

        let content = b"some_image_content".to_vec();

        // Store the record
        record.store(&pool, &mut notifier, Some(&content)).await?;

        // Load the record
        let loaded_record = AttachmentRecord::load(&pool, attachment_id).await?;
        assert_eq!(loaded_record.unwrap(), record);

        Ok(())
    }

    #[sqlx::test]
    async fn attachment_record_update_status(pool: Pool<Sqlite>) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let conversation = test_conversation();
        conversation
            .store(pool.acquire().await?.as_mut(), &mut notifier)
            .await?;
        let message = test_conversation_message(conversation.id());
        message.store(&pool, &mut notifier).await?;

        let attachment_id = AttachmentId::new(Uuid::new_v4());
        let arrived_at = Utc::now().round_subsecs(6);

        let record = AttachmentRecord {
            attachment_id,
            conversation_id: conversation.id(),
            conversation_message_id: message.id(),
            content_type: "image/png".to_string(),
            status: AttachmentStatus::Pending,
            arrived_at,
        };

        let content = b"some_image_content".to_vec();

        record.store(&pool, &mut notifier, Some(&content)).await?;
        let loaded_record = AttachmentRecord::load(&pool, attachment_id).await?;
        assert_eq!(loaded_record.unwrap(), record);

        AttachmentRecord::update_status(&pool, attachment_id, AttachmentStatus::Ready).await?;
        let loaded_record = AttachmentRecord::load(&pool, attachment_id).await?;
        assert_eq!(
            loaded_record.unwrap(),
            AttachmentRecord {
                status: AttachmentStatus::Ready,
                ..record
            }
        );

        Ok(())
    }
}
