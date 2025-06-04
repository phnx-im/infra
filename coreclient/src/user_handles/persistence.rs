// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{SubsecRound, Utc};
use phnxcommon::{
    codec::{BlobDecoded, BlobEncoded},
    credentials::keys::HandleSigningKey,
    identifiers::{UserHandle, UserHandleHash},
};
use sqlx::{SqliteExecutor, query};
use tracing::error;

/// A user handle record stored in the client database.
///
/// Contains additional information about the handle, such as hash and signature key.
pub(super) struct UserHandleRecord {
    pub(super) hash: UserHandleHash,
    pub(super) signature_key: HandleSigningKey,
}

struct SqlUserHandleRecord {
    hash: UserHandleHash,
    signature_key: BlobDecoded<HandleSigningKey>,
}

impl From<SqlUserHandleRecord> for UserHandleRecord {
    fn from(record: SqlUserHandleRecord) -> Self {
        Self {
            hash: record.hash,
            signature_key: record.signature_key.into_inner(),
        }
    }
}

impl UserHandleRecord {
    pub(super) async fn load(
        executor: impl SqliteExecutor<'_>,
        handle: &UserHandle,
    ) -> sqlx::Result<Option<Self>> {
        let plaintext = handle.plaintext();
        let record = sqlx::query_as!(
            SqlUserHandleRecord,
            r#"
                SELECT
                    hash AS "hash: _",
                    signature_key AS "signature_key: _"
                FROM user_handles
                WHERE handle = ?
            "#,
            plaintext
        )
        .fetch_optional(executor)
        .await?;
        Ok(record.map(From::from))
    }

    pub(super) async fn load_all_handles(
        executor: impl SqliteExecutor<'_>,
    ) -> sqlx::Result<Vec<UserHandle>> {
        let plaintext = sqlx::query_scalar!(
            r#"
                SELECT
                    handle
                FROM user_handles
            "#
        )
        .fetch_all(executor)
        .await?;
        Ok(plaintext
            .into_iter()
            .filter_map(|plaintext| {
                UserHandle::new(plaintext)
                    .inspect_err(
                        |error| error!(%error,"failed to parse user handle from plaintext"),
                    )
                    .ok()
            })
            .collect())
    }

    pub(super) async fn store(
        executor: impl SqliteExecutor<'_>,
        handle: &UserHandle,
        hash: &UserHandleHash,
        signing_key: &HandleSigningKey,
    ) -> sqlx::Result<()> {
        let plaintext = handle.plaintext();
        let signing_key = BlobEncoded(signing_key);
        let created_at = Utc::now().round_subsecs(6);
        let refreshed_at = created_at;
        query!(
            r#"
                INSERT INTO user_handles (
                    handle,
                    hash,
                    signature_key,
                    created_at,
                    refreshed_at
                ) VALUES (?, ?, ?, ?, ?)
            "#,
            plaintext,
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
        let plaintext = handle.plaintext();
        query!(
            r#"
                DELETE FROM user_handles
                WHERE handle = ?
            "#,
            plaintext,
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
