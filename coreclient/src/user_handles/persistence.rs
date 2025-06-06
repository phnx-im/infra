// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::Utc;
use phnxcommon::{
    codec::{BlobDecoded, BlobEncoded},
    credentials::keys::HandleSigningKey,
    identifiers::{UserHandle, UserHandleHash},
};
use sqlx::{SqliteExecutor, query, query_as, query_scalar};

/// A user handle record stored in the client database.
///
/// Contains additional information about the handle, such as hash and signature key.
pub struct UserHandleRecord {
    #[expect(dead_code)]
    pub handle: UserHandle,
    pub hash: UserHandleHash,
    pub signing_key: HandleSigningKey,
}

struct SqlUserHandleRecord {
    handle: UserHandle,
    hash: UserHandleHash,
    signing_key: BlobDecoded<HandleSigningKey>,
}

impl From<SqlUserHandleRecord> for UserHandleRecord {
    fn from(record: SqlUserHandleRecord) -> Self {
        Self {
            handle: record.handle,
            hash: record.hash,
            signing_key: record.signing_key.into_inner(),
        }
    }
}

impl UserHandleRecord {
    pub(super) async fn load(
        executor: impl SqliteExecutor<'_>,
        handle: &UserHandle,
    ) -> sqlx::Result<Option<Self>> {
        let record = query_as!(
            SqlUserHandleRecord,
            r#"
                SELECT
                    handle AS "handle: _",
                    hash AS "hash: _",
                    signing_key AS "signing_key: _"
                FROM user_handles
                WHERE handle = ?
            "#,
            handle
        )
        .fetch_optional(executor)
        .await?;
        Ok(record.map(From::from))
    }

    #[expect(dead_code)]
    pub(super) async fn load_all(executor: impl SqliteExecutor<'_>) -> sqlx::Result<Vec<Self>> {
        let records = query_as!(
            SqlUserHandleRecord,
            r#"
                SELECT
                    handle AS "handle: _",
                    hash AS "hash: _",
                    signing_key AS "signing_key: _"
                FROM user_handles
                ORDER BY created_at ASC
            "#,
        )
        .fetch_all(executor)
        .await?;
        Ok(records.into_iter().map(From::from).collect())
    }

    pub(super) async fn load_all_handles(
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<Vec<UserHandle>> {
        query_scalar!(
            r#"
                SELECT handle AS "handle: _"
                FROM user_handles
                ORDER BY created_at ASC
            "#
        )
        .fetch_all(executor)
        .await
    }

    pub(super) async fn store(
        executor: impl SqliteExecutor<'_>,
        handle: &UserHandle,
        hash: &UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> sqlx::Result<()> {
        let signing_key = BlobEncoded(signing_key);
        let created_at = Utc::now();
        let refreshed_at = created_at;
        query!(
            r#"
                INSERT INTO user_handles (
                    handle,
                    hash,
                    signing_key,
                    created_at,
                    refreshed_at
                ) VALUES (?, ?, ?, ?, ?)
            "#,
            handle,
            hash,
            signing_key,
            created_at,
            refreshed_at,
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub(super) async fn delete(
        executor: impl SqliteExecutor<'_>,
        handle: &UserHandle,
    ) -> sqlx::Result<()> {
        query!(
            r#"
                DELETE FROM user_handles
                WHERE handle = ?
            "#,
            handle,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use sqlx::SqlitePool;

    use super::*;

    #[sqlx::test]
    async fn user_handle_persistence(pool: SqlitePool) -> anyhow::Result<()> {
        let handle = UserHandle::new("ellie_03".to_owned())?;
        let hash = handle.hash()?;
        let signing_key = HandleSigningKey::generate()?;

        // Store a user handle
        UserHandleRecord::store(&pool, &handle, &hash, &signing_key).await?;

        // Load the user handle
        let loaded_handle = UserHandleRecord::load(&pool, &handle).await?.unwrap();
        assert_eq!(loaded_handle.hash, hash);

        // Load all handles (should only be one)
        let all_handles = UserHandleRecord::load_all_handles(&pool).await?;
        assert_eq!(all_handles.len(), 1);
        assert_eq!(all_handles[0], handle);

        // Delete the user handle
        UserHandleRecord::delete(&pool, &handle).await?;

        // Verify deletion
        let loaded_handle = UserHandleRecord::load(&pool, &handle).await?;
        assert!(loaded_handle.is_none());

        Ok(())
    }
}
