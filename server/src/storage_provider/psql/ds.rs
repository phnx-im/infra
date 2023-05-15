use async_trait::async_trait;
use chrono::{Duration, Utc};
use phnxbackend::{
    crypto::EncryptedDsGroupState,
    ds::{DsStorageProvider, LoadState},
    types::GroupId,
};
use sqlx::{types::uuid::Uuid as SqlxUuid, PgPool};
use uuid::Uuid;

/// A storage provider for the DS using PostgreSQL.
pub struct PgDsStorage {
    pool: PgPool,
}

#[async_trait]
impl DsStorageProvider for PgDsStorage {
    type StorageError = sqlx::Error;

    /// Create a new group.
    async fn create_group_state(
        &self,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<GroupId, sqlx::Error> {
        // Generate a new group ID.
        let group_id = Uuid::new_v4();

        // Execute the query
        match sqlx::query!(
            r#"
            INSERT INTO ds_group_states (id, encrypted_group_state, last_used)
            VALUES ($1, $2, $3)
            "#,
            SqlxUuid::from_u128(group_id.as_u128()),
            encrypted_group_state.ciphertext,
            Utc::now()
        )
        .execute(&self.pool)
        .await
        {
            Ok(_) => Ok(GroupId(group_id)),
            Err(e) => Err(e),
        }
    }

    /// Get a group's state.
    /// TODO: Expiration delay needs to be configurable.
    async fn load_group_state(&self, group_id: &GroupId) -> LoadState {
        match sqlx::query!(
            r#"
            SELECT encrypted_group_state, last_used
            FROM ds_group_states
            WHERE id = $1
            "#,
            SqlxUuid::from_u128(group_id.0.as_u128())
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => {
                if Utc::now().signed_duration_since(row.last_used) < Duration::days(90) {
                    LoadState::Success(EncryptedDsGroupState {
                        ciphertext: row.encrypted_group_state,
                        last_used: row.last_used,
                    })
                } else {
                    LoadState::Expired
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to load group state for group {:?} withe error {:?}",
                    group_id,
                    e
                );
                LoadState::NotFound
            }
        }
    }

    /// Save the ds group state with the group ID.
    async fn save_group_state(
        &self,
        _group_id: &GroupId,
        _encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }
}

impl PgDsStorage {
    /// Create a new PgDsStorage with a connection pool.
    pub async fn new(connection_string: &str) -> Self {
        let pool = PgPool::connect(connection_string)
            .await
            .expect("Failed to connect to database.");

        Self { pool }
    }
}
