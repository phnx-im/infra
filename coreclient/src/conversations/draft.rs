// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};

use crate::MessageId;

/// A message draft which is currently composed in a chat.
///
/// Allows to persists drafts between opening and closing the chat and between sessions of
/// the app.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct MessageDraft {
    /// The text currently composed in the draft.
    pub message: String,
    /// The id of the message currently being edited, if any.
    pub editing_id: Option<MessageId>,
    /// The time when the draft was last updated.
    pub updated_at: DateTime<Utc>,
}

mod persistence {
    use sqlx::{SqliteExecutor, query, query_as};

    use crate::{ChatId, store::StoreNotifier};

    use super::*;

    impl MessageDraft {
        pub(crate) async fn load(
            executor: impl SqliteExecutor<'_>,
            chat_id: ChatId,
        ) -> sqlx::Result<Option<Self>> {
            query_as!(
                MessageDraft,
                r#"
                    SELECT
                        message,
                        editing_id AS "editing_id: _",
                        updated_at AS "updated_at: _"
                    FROM message_draft
                    WHERE chat_id = ?
                "#,
                chat_id
            )
            .fetch_optional(executor)
            .await
        }

        pub(crate) async fn store(
            &self,
            executor: impl SqliteExecutor<'_>,
            notifier: &mut StoreNotifier,
            chat_id: ChatId,
        ) -> sqlx::Result<()> {
            query!(
                "INSERT OR REPLACE INTO message_draft (
                    chat_id,
                    message,
                    editing_id,
                    updated_at
                ) VALUES (?, ?, ?, ?)",
                chat_id,
                self.message,
                self.editing_id,
                self.updated_at,
            )
            .execute(executor)
            .await?;
            notifier.update(chat_id);
            Ok(())
        }

        pub(crate) async fn delete(
            executor: impl SqliteExecutor<'_>,
            notifier: &mut StoreNotifier,
            chat_id: ChatId,
        ) -> sqlx::Result<()> {
            query!("DELETE FROM message_draft WHERE chat_id = ?", chat_id)
                .execute(executor)
                .await?;
            notifier.update(chat_id);
            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use chrono::SubsecRound;
        use sqlx::SqlitePool;

        use crate::{
            conversations::{
                messages::persistence::tests::test_chat_message, persistence::tests::test_chat,
            },
            store::StoreNotifier,
        };

        use super::*;

        #[sqlx::test]
        async fn store_load_and_delete_message_draft(pool: SqlitePool) -> anyhow::Result<()> {
            let mut notifier = StoreNotifier::noop();

            let chat = test_chat();
            chat.store(pool.acquire().await?.as_mut(), &mut notifier)
                .await?;

            let message = test_chat_message(chat.id());
            message.store(&pool, &mut notifier).await?;

            // 1. Load non-existent draft (should be None)
            let loaded_draft = MessageDraft::load(&pool, chat.id()).await?;
            assert_eq!(loaded_draft, None);

            // 2. Store a new draft
            let now = Utc::now().round_subsecs(6); // Round to avoid precision issues with SQLite TEXT storage
            let draft = MessageDraft {
                message: "Hello, world!".to_string(),
                editing_id: Some(message.id()),
                updated_at: now,
            };
            draft.store(&pool, &mut notifier, chat.id()).await?;

            // 3. Load the stored draft and assert its contents
            let loaded_draft = MessageDraft::load(&pool, chat.id()).await?;
            assert!(loaded_draft.is_some());
            let loaded_draft = loaded_draft.unwrap();
            assert_eq!(loaded_draft.message, "Hello, world!".to_string());
            assert_eq!(loaded_draft.editing_id, draft.editing_id);
            assert_eq!(loaded_draft.updated_at, now);

            // 4. Update the draft and store again (INSERT OR REPLACE)
            let updated_now = Utc::now().round_subsecs(6);
            let updated_draft = MessageDraft {
                message: "Updated message.".to_string(),
                editing_id: None, // No longer editing
                updated_at: updated_now,
            };
            updated_draft.store(&pool, &mut notifier, chat.id()).await?;

            // 5. Load the updated draft and assert its new contents
            let loaded_draft = MessageDraft::load(&pool, chat.id()).await?;
            assert!(loaded_draft.is_some());
            let loaded_draft = loaded_draft.unwrap();
            assert_eq!(loaded_draft.message, "Updated message.");
            assert_eq!(loaded_draft.editing_id, None);
            assert_eq!(loaded_draft.updated_at, updated_now);

            // 6. Delete the draft
            MessageDraft::delete(&pool, &mut notifier, chat.id()).await?;

            // 7. Try to load it again (should be None)
            let loaded_draft_after_delete = MessageDraft::load(&pool, chat.id()).await?;
            assert_eq!(loaded_draft_after_delete, None);

            Ok(())
        }
    }
}
