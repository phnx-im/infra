// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    codec::{BlobDecoded, BlobEncoded},
    identifiers::{QualifiedGroupId, SealedClientReference},
};
use sqlx::{
    PgExecutor, query,
    types::chrono::{DateTime, Utc},
};

use crate::{ds::group_state::EncryptedDsGroupState, errors::StorageError};

use super::StorableDsGroupData;

impl StorableDsGroupData {
    pub(super) async fn store(&self, connection: impl PgExecutor<'_>) -> Result<(), StorageError> {
        query!(
            "INSERT INTO
                encrypted_groups
                (group_id, ciphertext, last_used, deleted_queues)
            VALUES
                ($1, $2, $3, $4)
            ON CONFLICT (group_id) DO NOTHING",
            self.group_id,
            BlobEncoded(&self.encrypted_group_state) as _,
            DateTime::<Utc>::from(self.last_used),
            BlobEncoded(&self.deleted_queues) as _,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn load(
        connection: impl PgExecutor<'_>,
        qgid: &QualifiedGroupId,
    ) -> Result<Option<Self>, StorageError> {
        let record = query!(
            r#"SELECT
                group_id,
                ciphertext AS "ciphertext: BlobDecoded<EncryptedDsGroupState>",
                last_used,
                deleted_queues AS "deleted_queues: BlobDecoded<Vec<SealedClientReference>>"
            FROM
                encrypted_groups
            WHERE
                group_id = $1"#,
            qgid.group_uuid()
        )
        .fetch_optional(connection)
        .await?;
        Ok(record.map(|record| Self {
            group_id: record.group_id,
            encrypted_group_state: record.ciphertext.into_inner(),
            last_used: record.last_used.into(),
            deleted_queues: record.deleted_queues.into_inner(),
        }))
    }

    pub(crate) async fn update(&self, connection: impl PgExecutor<'_>) -> Result<(), StorageError> {
        query!(
            "UPDATE
                encrypted_groups
            SET
                ciphertext = $2, last_used = $3, deleted_queues = $4
            WHERE
                group_id = $1",
            self.group_id,
            BlobEncoded(&self.encrypted_group_state) as _,
            DateTime::<Utc>::from(self.last_used),
            BlobEncoded(&self.deleted_queues) as _,
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        connection: impl PgExecutor<'_>,
        qgid: &QualifiedGroupId,
    ) -> Result<(), StorageError> {
        query!(
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
    use phnxtypes::{crypto::ear::Ciphertext, identifiers::QualifiedGroupId, time::TimeStamp};
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::{
        ds::{
            Ds,
            group_state::{EncryptedDsGroupState, StorableDsGroupData},
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

        let test_state = Ciphertext::dummy();

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
        storable_group_data.encrypted_group_state.flip_bit();

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

    async fn store_random_group(
        pool: &PgPool,
        ds: &Ds,
    ) -> anyhow::Result<(QualifiedGroupId, StorableDsGroupData)> {
        let group_uuid = Uuid::new_v4();
        let was_reserved = ds.reserve_group_id(group_uuid).await;
        assert!(was_reserved);

        let qgid = QualifiedGroupId::new(group_uuid, ds.own_domain.clone());
        let reserved_group_id = ds.claim_reserved_group_id(qgid.group_uuid()).await.unwrap();

        let group = random_group(reserved_group_id.0);
        group.store(pool).await?;

        Ok((qgid, group))
    }

    fn random_group(group_id: Uuid) -> StorableDsGroupData {
        StorableDsGroupData {
            group_id,
            encrypted_group_state: EncryptedDsGroupState::from(Ciphertext::random()),
            last_used: TimeStamp::now(),
            deleted_queues: vec![],
        }
    }

    #[sqlx::test]
    async fn load(pool: PgPool) -> anyhow::Result<()> {
        let ds = Ds::new_from_pool(pool.clone(), "example.com".parse().unwrap()).await?;
        let (qgid, group) = store_random_group(&pool, &ds).await?;

        let loaded = StorableDsGroupData::load(&pool, &qgid).await?;
        assert_eq!(loaded.unwrap(), group);

        Ok(())
    }

    #[sqlx::test]
    async fn update(pool: PgPool) -> anyhow::Result<()> {
        let ds = Ds::new_from_pool(pool.clone(), "example.com".parse().unwrap()).await?;
        let (qgid, group) = store_random_group(&pool, &ds).await?;

        let loaded = StorableDsGroupData::load(&pool, &qgid).await?;
        assert_eq!(loaded.unwrap(), group);

        let updated_group = random_group(group.group_id);
        updated_group.update(&pool).await?;

        let loaded = StorableDsGroupData::load(&pool, &qgid).await?;
        assert_eq!(loaded.unwrap(), updated_group);

        Ok(())
    }

    #[sqlx::test]
    async fn delete(pool: PgPool) -> anyhow::Result<()> {
        let ds = Ds::new_from_pool(pool.clone(), "example.com".parse().unwrap()).await?;
        let (qgid, group) = store_random_group(&pool, &ds).await?;

        let loaded = StorableDsGroupData::load(&pool, &qgid).await?;
        assert_eq!(loaded.unwrap(), group);

        StorableDsGroupData::delete(&pool, &qgid).await?;

        let loaded = StorableDsGroupData::load(&pool, &qgid).await?;
        assert!(loaded.is_none());

        Ok(())
    }
}
