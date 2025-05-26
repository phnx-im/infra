// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use sqlx::PgPool;

use super::{Allowance, RLKey, StorageProvider};

pub(crate) struct RLPostgresStorage {
    pool: PgPool,
}

impl RLPostgresStorage {
    pub(crate) fn new(pool: PgPool) -> Self {
        RLPostgresStorage { pool }
    }
}

#[async_trait]
impl StorageProvider for RLPostgresStorage {
    async fn get(&self, key: &RLKey) -> Option<Allowance> {
        Allowance::load(&self.pool, key).await.ok().flatten()
    }

    async fn set(&self, key: RLKey, allowance: Allowance) {
        if let Err(e) = allowance.store(&self.pool, &key).await {
            tracing::error!(%e, "Failed to store allowance in Postgres");
        }
    }
}

pub(crate) mod persistence {

    use sqlx::{
        PgExecutor, query, query_as,
        types::chrono::{DateTime, Utc},
    };

    use crate::{errors::StorageError, rate_limiter::RLKey};

    use super::Allowance;

    impl Allowance {
        /// Load an Allowance from the database by its key.
        pub(in crate::rate_limiter) async fn load(
            connection: impl PgExecutor<'_>,
            key: &RLKey,
        ) -> Result<Option<Allowance>, StorageError> {
            struct AllowanceRecord {
                remaining: i64,
                valid_until: DateTime<Utc>,
            }

            let record = query_as!(
                AllowanceRecord,
                r#"SELECT
                    remaining AS "remaining: _",
                    valid_until AS "valid_until: _"
                FROM allowance_records
                WHERE key_value = $1"#,
                key.serialize(),
            )
            .fetch_optional(connection)
            .await?;
            Ok(record.map(|record| Allowance {
                remaining: record.remaining as u64,
                valid_until: record.valid_until,
            }))
        }

        /// Store an Allowance in the database.
        pub(in crate::rate_limiter) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
            key: &RLKey,
        ) -> Result<(), StorageError> {
            query!(
                "INSERT INTO allowance_records
                    (key_value, remaining, valid_until)
                    VALUES ($1, $2, $3)",
                key.serialize(),
                self.remaining as i64,
                DateTime::<Utc>::from(self.valid_until),
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        /// Delete all expried allowances.
        #[allow(dead_code)]
        pub(in crate::rate_limiter) async fn delete_expired(
            connection: impl PgExecutor<'_>,
        ) -> Result<(), sqlx::Error> {
            query!("DELETE FROM allowance_records WHERE valid_until < NOW()")
                .execute(connection)
                .await?;
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) mod tests {
        use chrono::TimeDelta;
        use sqlx::PgPool;

        use super::*;

        pub async fn store_random_allowance(
            pool: &PgPool,
            key: &RLKey,
        ) -> anyhow::Result<Allowance> {
            let allowance = Allowance {
                remaining: 10,
                valid_until: Utc::now() + TimeDelta::hours(1),
            };
            allowance.store(pool, key).await?;
            Ok(allowance)
        }

        #[sqlx::test]
        async fn load_allowance(pool: PgPool) -> anyhow::Result<()> {
            let key = RLKey::new("test_service", "test_rpc", &[]);
            let allowance = store_random_allowance(&pool, &key).await?;

            let loaded = Allowance::load(&pool, &key)
                .await?
                .expect("missing allowance record");
            assert_eq!(loaded, allowance);

            Ok(())
        }

        #[sqlx::test]
        async fn delete_expired_allowances(pool: PgPool) -> anyhow::Result<()> {
            // First, store an allowance that is valid
            let key = RLKey::new("test_service", "test_rpc", &[]);
            let allowance = store_random_allowance(&pool, &key).await?;

            // Then, delete expired allowances (should not delete the valid one)
            Allowance::delete_expired(&pool).await?;

            // Load the valid allowance to ensure it still exists
            let loaded = Allowance::load(&pool, &key)
                .await?
                .expect("missing allowance record");
            assert_eq!(loaded, allowance);

            // Now, store an expired allowance
            let expired_key = RLKey::new("expired_service", "expired_rpc", &[]);
            let expired_allowance = Allowance {
                remaining: 0,
                valid_until: Utc::now() - TimeDelta::weeks(1), // already expired
            };
            expired_allowance.store(&pool, &expired_key).await?;

            // Delete expired allowances again
            Allowance::delete_expired(&pool).await?;

            // Ensure the expired allowance is deleted
            let loaded_expired = Allowance::load(&pool, &expired_key).await?;
            assert!(loaded_expired.is_none());

            Ok(())
        }
    }
}
