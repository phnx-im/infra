// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::PgPool;

use super::{Allowance, RlKey, StorageProvider};

pub(crate) struct RLPostgresStorage {
    pool: PgPool,
}

impl RLPostgresStorage {
    pub(crate) fn new(pool: PgPool) -> Self {
        RLPostgresStorage { pool }
    }
}

impl StorageProvider for RLPostgresStorage {
    async fn get(&self, key: &RlKey) -> Option<Allowance> {
        Allowance::load(&self.pool, key).await.ok().flatten()
    }

    async fn set(&self, key: RlKey, allowance: Allowance) {
        if let Err(error) = allowance.store(&self.pool, &key).await {
            tracing::error!(%error, "Failed to store allowance in Postgres");
        }
    }
}

pub(crate) mod persistence {

    use chrono::{SubsecRound, Timelike};
    use sqlx::{
        PgExecutor, query, query_as,
        types::chrono::{DateTime, Utc},
    };

    use crate::{errors::StorageError, rate_limiter::RlKey};

    use super::Allowance;

    /// Drop the last three digits so the value really is µs-precise.
    fn trunc_to_micros(dt: DateTime<Utc>) -> DateTime<Utc> {
        let micros = dt.timestamp_subsec_micros();
        dt.with_nanosecond(micros * 1_000).unwrap()
    }

    impl Allowance {
        /// Load an Allowance from the database by its key.
        pub(in crate::rate_limiter) async fn load(
            connection: impl PgExecutor<'_>,
            key: &RlKey,
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
            key: &RlKey,
        ) -> Result<(), StorageError> {
            query!(
                "INSERT INTO allowance_records
                    (key_value, remaining, valid_until)
                    VALUES ($1, $2, $3)",
                key.serialize(),
                self.remaining as i64,
                trunc_to_micros(self.valid_until),
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

        #[test]
        fn trunc_to_micros_test() {
            let dt = DateTime::<Utc>::from_timestamp(1_000_000, 123_456_789).unwrap();
            assert_eq!(dt.timestamp(), 1_000_000);
            assert_eq!(dt.timestamp_subsec_nanos(), 123_456_789);
            let truncated = trunc_to_micros(dt);
            assert_eq!(truncated.timestamp(), 1_000_000);
            assert_eq!(truncated.timestamp_subsec_micros(), 123_456);
            assert_eq!(truncated.timestamp_subsec_nanos(), 123_456_000);
        }

        pub async fn store_random_allowance(
            pool: &PgPool,
            key: &RlKey,
        ) -> anyhow::Result<Allowance> {
            let allowance = Allowance {
                remaining: 10,
                valid_until: trunc_to_micros(Utc::now() + TimeDelta::hours(1)),
            };
            allowance.store(pool, key).await?;
            Ok(allowance)
        }

        #[sqlx::test]
        async fn load_allowance(pool: PgPool) -> anyhow::Result<()> {
            let key = RlKey::new(b"test_service", b"test_rpc", &[]);
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
            let key = RlKey::new(b"test_service", b"test_rpc", &[]);
            let allowance = store_random_allowance(&pool, &key).await?;

            // Then, delete expired allowances (should not delete the valid one)
            Allowance::delete_expired(&pool).await?;

            // Load the valid allowance to ensure it still exists
            let loaded = Allowance::load(&pool, &key)
                .await?
                .expect("missing allowance record");
            assert_eq!(loaded, allowance);

            // Now, store an expired allowance
            let expired_key = RlKey::new(b"expired_service", b"expired_rpc", &[]);
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
