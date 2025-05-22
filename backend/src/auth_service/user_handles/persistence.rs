use sqlx::{PgExecutor, PgPool, query, query_scalar};

use super::*;

pub(crate) struct UserHandleRecord {
    pub(crate) user_handle_hash: UserHandleHash,
    pub(crate) verifying_key: HandleVerifyingKey,
    pub(crate) expiration_data: ExpirationData,
}

impl UserHandleRecord {
    pub(crate) async fn store(&self, executor: impl PgExecutor<'_>) -> sqlx::Result<bool> {
        let res = query!(
            "INSERT INTO as_user_handles (
                hash,
                verifying_key,
                expiration_data
            ) VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING",
            self.user_handle_hash.as_bytes(),
            self.verifying_key as _,
            self.expiration_data as _,
        )
        .execute(executor)
        .await?;
        let inserted = res.rows_affected() > 0;
        Ok(inserted)
    }

    pub(crate) async fn load_verifying_key(
        executor: impl PgExecutor<'_>,
        hash: &UserHandleHash,
    ) -> sqlx::Result<Option<HandleVerifyingKey>> {
        query_scalar!(
            r#"SELECT verifying_key AS "verifying_key: HandleVerifyingKey"
                FROM as_user_handles WHERE hash = $1"#,
            hash.as_bytes(),
        )
        .fetch_optional(executor)
        .await
    }

    pub(super) async fn delete(
        executor: impl PgExecutor<'_>,
        hash: UserHandleHash,
    ) -> sqlx::Result<bool> {
        let res = query!(
            "DELETE FROM as_user_handles WHERE hash = $1",
            hash.as_bytes(),
        )
        .execute(executor)
        .await?;
        let deleted = res.rows_affected() > 0;
        Ok(deleted)
    }

    #[cfg(test)]
    async fn load_expiration_data(
        executor: impl PgExecutor<'_>,
        hash: UserHandleHash,
    ) -> sqlx::Result<Option<ExpirationData>> {
        query_scalar!(
            r#"SELECT expiration_data AS "expiration_data: ExpirationData"
            FROM as_user_handles WHERE hash = $1"#,
            hash.as_bytes(),
        )
        .fetch_optional(executor)
        .await
    }

    pub(super) async fn update_expiration_data(
        db_pool: &PgPool,
        hash: UserHandleHash,
        expiration_data: ExpirationData,
    ) -> sqlx::Result<bool> {
        let res = query!(
            "UPDATE as_user_handles SET expiration_data = $1 WHERE hash = $2",
            expiration_data as _,
            hash.as_bytes(),
        )
        .execute(db_pool)
        .await?;
        let updated = res.rows_affected() > 0;
        Ok(updated)
    }
}

#[cfg(test)]
mod test {
    use phnxtypes::time::Duration;
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_store_and_load_user_handle_record(pool: PgPool) -> anyhow::Result<()> {
        let user_handle_hash = UserHandleHash::new([1; 32]);
        let verifying_key = HandleVerifyingKey::from_bytes(vec![1, 2, 3, 4, 5]);
        let expiration_data = ExpirationData::new(Duration::days(1));

        let record = UserHandleRecord {
            user_handle_hash,
            verifying_key: verifying_key.clone(),
            expiration_data: expiration_data.clone(),
        };

        // Test storing a new record
        let inserted = record.store(&pool).await?;
        assert!(inserted, "Record should be inserted successfully");

        // Test loading the verifying key
        let loaded_verifying_key =
            UserHandleRecord::load_verifying_key(&pool, &user_handle_hash).await?;
        assert_eq!(
            loaded_verifying_key,
            Some(verifying_key.clone()),
            "Loaded verifying key should match"
        );

        // Test storing the same record again (ON CONFLICT DO NOTHING)
        let inserted_again = record.store(&pool).await?;
        assert!(!inserted_again, "Duplicate record should not be inserted");

        // Test loading a non-existent key
        let non_existent_hash = UserHandleHash::new([2; 32]);
        let loaded_non_existent =
            UserHandleRecord::load_verifying_key(&pool, &non_existent_hash).await?;
        assert_eq!(
            loaded_non_existent, None,
            "Loading non-existent key should return None"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_user_handle_record(pool: PgPool) -> anyhow::Result<()> {
        let user_handle_hash = UserHandleHash::new([1; 32]);
        let verifying_key = HandleVerifyingKey::from_bytes(vec![1, 2, 3, 4, 5]);
        let expiration_data = ExpirationData::new(Duration::days(1));

        let record = UserHandleRecord {
            user_handle_hash,
            verifying_key,
            expiration_data,
        };

        // Store the record first
        record.store(&pool).await?;

        // Test deleting an existing record
        let deleted = UserHandleRecord::delete(&pool, user_handle_hash).await?;
        assert!(deleted, "Record should be deleted successfully");

        // Verify it's gone
        let loaded_after_delete =
            UserHandleRecord::load_verifying_key(&pool, &user_handle_hash).await?;
        assert_eq!(
            loaded_after_delete, None,
            "Record should not exist after deletion"
        );

        // Test deleting a non-existent record
        let non_existent_hash = UserHandleHash::new([2; 32]);
        let deleted_non_existent = UserHandleRecord::delete(&pool, non_existent_hash).await?;
        assert!(
            !deleted_non_existent,
            "Deleting non-existent record should return false"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_expiration_data(pool: PgPool) -> anyhow::Result<()> {
        let user_handle_hash = UserHandleHash::new([1; 32]);
        let verifying_key = HandleVerifyingKey::from_bytes(vec![1, 2, 3, 4, 5]);
        let initial_expiration_data = ExpirationData::new(Duration::days(1));
        let updated_expiration_data = ExpirationData::new(Duration::days(2));

        let record = UserHandleRecord {
            user_handle_hash,
            verifying_key: verifying_key.clone(),
            expiration_data: initial_expiration_data.clone(),
        };

        // Store the record first
        record.store(&pool).await?;

        let updated = UserHandleRecord::update_expiration_data(
            &pool,
            user_handle_hash,
            updated_expiration_data.clone(),
        )
        .await?;
        assert!(updated, "Expiration data should be updated successfully");

        // Verify the expiration data has been updated
        let loaded_expiration_data =
            UserHandleRecord::load_expiration_data(&pool, user_handle_hash)
                .await?
                .unwrap();
        assert_eq!(
            loaded_expiration_data, updated_expiration_data,
            "Expiration data should be updated"
        );

        let loaded_verifying_key = UserHandleRecord::load_verifying_key(&pool, &user_handle_hash)
            .await?
            .unwrap();
        assert_eq!(
            loaded_verifying_key, verifying_key,
            "Verifying key should remain unchanged"
        );

        // Test updating a non-existent record
        let non_existent_hash = UserHandleHash::new([2; 32]);
        let updated_non_existent = UserHandleRecord::update_expiration_data(
            &pool,
            non_existent_hash,
            ExpirationData::new(Duration::days(1)),
        )
        .await?;
        assert!(
            !updated_non_existent,
            "Updating non-existent record should return false"
        );

        Ok(())
    }
}
