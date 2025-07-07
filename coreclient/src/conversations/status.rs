// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use enumset::EnumSetType;
use mimi_content::MessageStatus;

pub(crate) struct StatusRecord {}

#[derive(Debug, EnumSetType)]
pub enum MessageStatusBit {
    Unread = 0,
    Delivered = 1,
    Read = 2,
    Expired = 3,
    Deleted = 4,
    Hidden = 5,
    Error = 6,
}

impl TryFrom<MessageStatus> for MessageStatusBit {
    type Error = ();

    fn try_from(value: MessageStatus) -> Result<Self, ()> {
        match value {
            MessageStatus::Unread => Ok(Self::Unread),
            MessageStatus::Delivered => Ok(Self::Delivered),
            MessageStatus::Read => Ok(Self::Read),
            MessageStatus::Expired => Ok(Self::Expired),
            MessageStatus::Deleted => Ok(Self::Deleted),
            MessageStatus::Hidden => Ok(Self::Hidden),
            MessageStatus::Error => Ok(Self::Error),
            MessageStatus::Custom(_) => Err(()),
        }
    }
}

impl From<MessageStatusBit> for MessageStatus {
    fn from(value: MessageStatusBit) -> Self {
        match value {
            MessageStatusBit::Unread => Self::Unread,
            MessageStatusBit::Delivered => Self::Delivered,
            MessageStatusBit::Read => Self::Read,
            MessageStatusBit::Expired => Self::Expired,
            MessageStatusBit::Deleted => Self::Deleted,
            MessageStatusBit::Hidden => Self::Hidden,
            MessageStatusBit::Error => Self::Error,
        }
    }
}

mod persistence {
    use std::collections::HashMap;

    use enumset::EnumSet;
    use mimi_content::{MessageStatusReport, PerMessageStatus};
    use phnxcommon::{identifiers::UserId, time::TimeStamp};
    use sqlx::{SqliteTransaction, query, query_scalar};
    use tracing::warn;

    use crate::{ConversationMessageId, store::StoreNotifier};

    use super::*;

    impl StatusRecord {
        pub(crate) async fn store_report(
            txn: &mut SqliteTransaction<'_>,
            notifier: &mut StoreNotifier,
            sender: &UserId,
            report: MessageStatusReport,
            created_at: TimeStamp,
        ) -> sqlx::Result<()> {
            let sender_uuid = sender.uuid();
            let sender_domain = sender.domain();

            // Group statuses by mimi id into an enum set
            let mut statuses: HashMap<&[u8], EnumSet<MessageStatusBit>> = Default::default();
            for PerMessageStatus { mimi_id, status } in &report.statuses {
                let mimi_id = mimi_id.as_slice();
                let Ok(bit) = (*status).try_into() else {
                    warn!(?status, "Unsupported message status");
                    continue;
                };
                statuses.entry(mimi_id).or_default().insert(bit);
            }

            // Store the message status for each mimi id
            for (mimi_id, status_bitset) in statuses {
                // Load the message id
                let Some(message_id) = query_scalar!(
                    r#"SELECT message_id AS "message_id: ConversationMessageId"
                        FROM conversation_messages
                        WHERE mimi_id = ?"#,
                    mimi_id,
                )
                .fetch_optional(&mut **txn)
                .await?
                else {
                    continue;
                };

                let bitset = status_bitset.as_u32();

                // Set the statuses for the message and user
                query!(
                    "INSERT INTO conversation_message_status
                        (message_id,  sender_user_uuid, sender_user_domain, status_bitset, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    ON CONFLICT (message_id, sender_user_domain, sender_user_uuid)
                    DO UPDATE SET status_bitset = ?4, created_at = ?5",
                    message_id,
                    sender_uuid,
                    sender_domain,
                    bitset,
                    created_at,
                )
                .execute(&mut **txn)
                .await?;

                // Aggregate the status bitset for the message
                query!(
                    "UPDATE conversation_messages
                    SET status_bitset = status_bitset | ?1
                    WHERE message_id = ?2",
                    bitset,
                    message_id,
                )
                .execute(&mut **txn)
                .await?;

                notifier.update(message_id);
            }

            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use chrono::Utc;
        use mimi_content::MessageStatus;
        use rand::seq::SliceRandom;
        use sqlx::{SqlitePool, query_scalar};

        use crate::{
            ConversationMessage,
            conversations::{
                messages::persistence::tests::test_conversation_message_with_salt,
                persistence::tests::test_conversation,
            },
        };

        use super::*;

        #[sqlx::test]
        async fn store_report(pool: SqlitePool) -> anyhow::Result<()> {
            let mut notifier = StoreNotifier::noop();

            let alice = UserId::random("localhost".parse().unwrap());

            let conversation = test_conversation();
            conversation
                .store(pool.acquire().await?.as_mut(), &mut notifier)
                .await?;

            let message_a = test_conversation_message_with_salt(conversation.id(), [0; 16]);
            message_a.store(&pool, &mut notifier).await?;
            let mimi_id_a = message_a.message().mimi_id().unwrap();
            let message_b = test_conversation_message_with_salt(conversation.id(), [1; 16]);
            message_b.store(&pool, &mut notifier).await?;
            let mimi_id_b = message_b.message().mimi_id().unwrap();
            assert_ne!(mimi_id_a, mimi_id_b);

            let mut report = MessageStatusReport {
                statuses: Vec::new(),
            };

            use MessageStatus::*;
            for status in [
                Unread,
                Delivered,
                Read,
                Expired,
                Deleted,
                Hidden,
                Error,
                Custom(100),
            ] {
                report.statuses.push(PerMessageStatus {
                    mimi_id: mimi_id_a.as_ref().to_vec().into(),
                    status,
                });
                report.statuses.push(PerMessageStatus {
                    mimi_id: mimi_id_b.as_ref().to_vec().into(),
                    status,
                });
            }
            report.statuses.shuffle(&mut rand::thread_rng());

            let mut txn = pool.begin().await?;
            StatusRecord::store_report(&mut txn, &mut notifier, &alice, report, Utc::now().into())
                .await?;
            txn.commit().await?;

            for message in [message_a, message_b] {
                let message_id = message.id();
                let bits: i64 = query_scalar(
                    "SELECT status_bitset FROM conversation_message_status
                    WHERE message_id = ?",
                )
                .bind(message_id)
                .fetch_one(&mut *pool.acquire().await?)
                .await?;
                let bitset: EnumSet<MessageStatusBit> = EnumSet::from_u32_truncated(bits as u32);
                assert_eq!(bitset, EnumSet::all());

                let message = ConversationMessage::load(&pool, message.id())
                    .await?
                    .unwrap();
                assert_eq!(message.status(), EnumSet::all());
            }

            Ok(())
        }
    }
}
