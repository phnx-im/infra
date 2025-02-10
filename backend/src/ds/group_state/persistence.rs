// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    codec::persist::{BlobPersist, BlobPersisted},
    identifiers::{QualifiedGroupId, SealedClientReference},
};
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgExecutor,
};
use uuid::Uuid;

use crate::{ds::group_state::EncryptedDsGroupState, errors::StorageError};

use super::StorableDsGroupData;

impl StorableDsGroupData {
    pub(super) async fn store(&self, connection: impl PgExecutor<'_>) -> Result<(), StorageError> {
        sqlx::query!(
            "INSERT INTO
                encrypted_groups
                (group_id, ciphertext, last_used, deleted_queues)
            VALUES
                ($1, $2, $3, $4)
            ON CONFLICT (group_id) DO NOTHING",
            self.group_id,
            self.encrypted_group_state.persist() as _,
            DateTime::<Utc>::from(self.last_used),
            self.deleted_queues.persist() as _,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn load(
        connection: impl PgExecutor<'_>,
        qgid: &QualifiedGroupId,
    ) -> Result<Option<StorableDsGroupData>, StorageError> {
        struct SqlStorableDsGroupData {
            group_id: Uuid,
            ciphertext: BlobPersisted<EncryptedDsGroupState>,
            last_used: DateTime<Utc>,
            deleted_queues: BlobPersisted<Vec<SealedClientReference>>,
        }

        let group_data = sqlx::query_as!(
            SqlStorableDsGroupData,
            r#"SELECT
                group_id,
                ciphertext AS "ciphertext: _",
                last_used,
                deleted_queues AS "deleted_queues: _"
            FROM
                encrypted_groups
            WHERE
                group_id = $1"#,
            qgid.group_uuid()
        )
        .fetch_optional(connection)
        .await?
        .map(
            |SqlStorableDsGroupData {
                 group_id,
                 ciphertext: BlobPersisted(encrypted_group_state),
                 last_used,
                 deleted_queues: BlobPersisted(deleted_queues),
             }| {
                Self {
                    group_id,
                    encrypted_group_state,
                    last_used: last_used.into(),
                    deleted_queues,
                }
            },
        );
        Ok(group_data)
    }

    pub(crate) async fn update(&self, connection: impl PgExecutor<'_>) -> Result<(), StorageError> {
        sqlx::query!(
            "UPDATE
                encrypted_groups
            SET
                ciphertext = $2,
                last_used = $3,
                deleted_queues = $4
            WHERE
                group_id = $1",
            self.group_id,
            self.encrypted_group_state.persist() as _,
            DateTime::<Utc>::from(self.last_used),
            self.deleted_queues.persist() as _,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        connection: impl PgExecutor<'_>,
        qgid: &QualifiedGroupId,
    ) -> Result<(), StorageError> {
        sqlx::query!(
            "DELETE FROM
                encrypted_groups
            WHERE
                group_id = $1",
            qgid.group_uuid()
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use phnxtypes::{crypto::ear::Ciphertext, identifiers::QualifiedGroupId};
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::{
        ds::{
            group_state::{EncryptedDsGroupState, StorableDsGroupData},
            Ds,
        },
        infra_service::InfraService,
    };

    #[sqlx::test]
    async fn reserve_group_id(pool: PgPool) {
        let ds = Ds::new_from_pool(pool, "example.com".parse().unwrap())
            .await
            .expect("Error creating ephemeral Ds instance.");

        // Sample a random group id and reserve it
        let group_uuid = Uuid::new_v4();

        let was_reserved = ds.reserve_group_id(group_uuid).await;
        assert!(was_reserved);

        // Try to reserve the same group id again
        let was_reserved_again = ds.reserve_group_id(group_uuid).await;

        // This should return false
        assert!(!was_reserved_again);
    }

    #[sqlx::test]
    async fn group_state_lifecycle(pool: PgPool) {
        let ds = Ds::new_from_pool(pool, "example.com".parse().unwrap())
            .await
            .expect("Error creating ephemeral Ds instance.");

        let dummy_ciphertext = Ciphertext::dummy();
        let test_state: EncryptedDsGroupState = dummy_ciphertext.into();

        // Create/store a dummy group state
        let group_uuid = Uuid::new_v4();
        let was_reserved = ds.reserve_group_id(group_uuid).await;
        assert!(was_reserved);

        // Load the reserved group id
        let qgid = QualifiedGroupId::new(group_uuid, ds.own_domain.clone());
        let reserved_group_id = ds.claim_reserved_group_id(qgid.group_uuid()).await.unwrap();

        // Create and store a new group state
        let mut storable_group_data =
            StorableDsGroupData::new_and_store(&ds.db_pool, reserved_group_id, test_state.clone())
                .await
                .unwrap();

        // Load the group state again
        let loaded_group_state = StorableDsGroupData::load(&ds.db_pool, &qgid)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            loaded_group_state.encrypted_group_state,
            storable_group_data.encrypted_group_state
        );

        // Update that group state.
        storable_group_data.encrypted_group_state.0.flip_bit();

        storable_group_data.update(&ds.db_pool).await.unwrap();

        // Load the group state again
        let loaded_group_state = StorableDsGroupData::load(&ds.db_pool, &qgid)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            loaded_group_state.encrypted_group_state,
            storable_group_data.encrypted_group_state
        );
    }
}
