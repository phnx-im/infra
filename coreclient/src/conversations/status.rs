// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) struct StatusRecord {}

mod persistence {
    use mimi_content::{MessageStatusReport, PerMessageStatus};
    use phnxcommon::{identifiers::UserId, time::TimeStamp};
    use sqlx::{SqliteTransaction, query, query_scalar};

    use crate::{ConversationMessageId, store::StoreNotifier};

    use super::*;

    impl StatusRecord {
        pub(crate) async fn store_report(
            txn: &mut SqliteTransaction<'_>,
            notifier: &mut StoreNotifier,
            sender: &UserId,
            mut report: MessageStatusReport,
            created_at: TimeStamp,
        ) -> sqlx::Result<()> {
            let sender_uuid = sender.uuid();
            let sender_domain = sender.domain();

            // Sort statuses for partitioning by mimi id
            report
                .statuses
                .sort_unstable_by(|a, b| a.mimi_id.cmp(&b.mimi_id));

            let mut current_mimi_id = None;
            let mut current_message_id: Option<ConversationMessageId> = None;

            for PerMessageStatus { mimi_id, status } in report.statuses {
                // Load message id if mimi id has changed
                if current_mimi_id.as_ref() != Some(&mimi_id) {
                    let mimi_id_slice = mimi_id.as_slice();
                    current_message_id = query_scalar!(
                        r#"SELECT message_id AS "message_id: _"
                        FROM conversation_messages
                        WHERE mimi_id = ?"#,
                        mimi_id_slice
                    )
                    .fetch_optional(&mut **txn)
                    .await?;

                    current_mimi_id = Some(mimi_id);
                }

                let Some(message_id) = current_message_id else {
                    continue;
                };

                let repr = status.repr();
                query!(
                    "INSERT INTO conversation_message_status
                        (message_id,  sender_user_uuid, sender_user_domain, status, created_at)
                    VALUES (?, ?, ?, ?, ?)",
                    message_id,
                    sender_uuid,
                    sender_domain,
                    repr,
                    created_at,
                )
                .execute(&mut **txn)
                .await?;

                notifier.update(message_id);
            }

            Ok(())
        }
    }
}
