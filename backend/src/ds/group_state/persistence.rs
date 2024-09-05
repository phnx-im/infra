// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::codec::PhnxCodec;
use phnxtypes::identifiers::QualifiedGroupId;
use sqlx::{
    types::chrono::{DateTime, Utc},
    Executor, PgConnection, PgExecutor,
};
use thiserror::Error;

use crate::ds::GROUP_STATE_EXPIRATION;

use super::{EncryptedDsGroupState, ReservedGroupId, StorableDsGroupData};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxtypes::codec::Error),
}

/// Return value of a group state load query.
#[derive(Debug)]
pub(crate) enum LoadResult {
    Success(StorableDsGroupData),
    NotFound,
    Expired,
}

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
            PhnxCodec::to_vec(&self.encrypted_group_state)?,
            DateTime::<Utc>::from(self.last_used),
            PhnxCodec::to_vec(&self.deleted_queues)?
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn load(
        qgid: &QualifiedGroupId,
        connection: impl PgExecutor<'_>,
    ) -> Result<Option<StorableDsGroupData>, StorageError> {
        sqlx::query!(
            "SELECT 
                group_id, ciphertext, last_used, deleted_queues
            FROM 
                encrypted_groups
            WHERE 
                group_id = $1",
            qgid.group_uuid()
        )
        .fetch_optional(connection)
        .await?;
        todo!()
        //    let Some(model) = EncryptedGroupData::find_by_id(qgid.group_uuid())
        //        .one(connection)
        //        .await?
        //    else {
        //        return Ok(LoadResult::NotFound);
        //    };
        //    let group_data = Self::try_from(model)?;

        //    if group_data.last_used.has_expired(GROUP_STATE_EXPIRATION) {
        //        return Ok(LoadResult::Expired);
        //    }

        //    if group_data.encrypted_group_state == EncryptedDsGroupState::default() {
        //        return Ok(LoadResult::Reserved(ReservedGroupId(group_data.group_id)));
        //    }

        //    Ok(LoadResult::Success(group_data))
    }

    pub(crate) async fn update<'a>(
        &self,
        connection: impl PgExecutor<'a>,
    ) -> Result<(), StorageError> {
        todo!()
    }

    pub(crate) async fn delete<'a>(
        qgid: &QualifiedGroupId,
        connection: impl PgExecutor<'a>,
    ) -> Result<(), StorageError> {
        todo!()
    }
}

//#[cfg(test)]
//mod test {
//    use phnxtypes::{
//        crypto::ear::Ciphertext,
//        identifiers::{Fqdn, QualifiedGroupId},
//    };
//    use uuid::Uuid;
//
//    use crate::ds::{
//        group_state::{
//            persistence::LoadResult, EncryptedDsGroupState, ReservedGroupId, StorableDsGroupData,
//        },
//        Ds,
//    };
//
//    #[tokio::test]
//    async fn reserve_group_id() {
//        let ds = Ds::new_ephemeral(Fqdn::try_from("example.com").unwrap())
//            .await
//            .expect("Error creating ephemeral Ds instance.");
//
//        // Sample a random group id and reserve it
//        let group_uuid = Uuid::new_v4();
//        let was_reserved = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
//            .await
//            .unwrap();
//        assert!(was_reserved);
//
//        // Try to reserve the same group id again
//        let was_reserved_again = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
//            .await
//            .expect("Error reserving group id.");
//
//        // This should return false
//        assert!(!was_reserved_again);
//    }
//
//    #[tokio::test]
//    async fn group_state_lifecycle() {
//        let ds = Ds::new_ephemeral(Fqdn::try_from("example.com").unwrap())
//            .await
//            .expect("Error creating ephemeral Ds instance.");
//
//        let dummy_ciphertext = Ciphertext::dummy();
//        let test_state: EncryptedDsGroupState = dummy_ciphertext.into();
//
//        // Create/store a dummy group state
//        let group_uuid = Uuid::new_v4();
//        let was_reserved = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
//            .await
//            .unwrap();
//        assert!(was_reserved);
//
//        // Load the reserved group id
//        let qgid = QualifiedGroupId::new(group_uuid, ds.own_domain.clone());
//        let LoadResult::Reserved(reserved_group_id) =
//            StorableDsGroupData::load(&qgid, &ds.db_connection)
//                .await
//                .unwrap()
//        else {
//            panic!("Error loading group state.");
//        };
//
//        let mut storable_group_data =
//            StorableDsGroupData::new(reserved_group_id, test_state.clone());
//
//        // Save the group state
//        storable_group_data
//            .update(&ds.db_connection)
//            .await
//            .expect("Error saving group state.");
//
//        // Load the group state again
//        let loaded_group_state = StorableDsGroupData::load(&qgid, &ds.db_connection)
//            .await
//            .unwrap();
//
//        if let LoadResult::Success(loaded_group_state) = loaded_group_state {
//            assert_eq!(
//                loaded_group_state.encrypted_group_state,
//                storable_group_data.encrypted_group_state
//            );
//        } else {
//            panic!("Error loading group state.");
//        }
//
//        // Try to reserve the group id of the created group state
//        let successfully_reserved = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
//            .await
//            .unwrap();
//
//        // This should return false
//        assert!(!successfully_reserved);
//
//        // Update that group state.
//        storable_group_data.encrypted_group_state.0.flip_bit();
//
//        storable_group_data.update(&ds.db_connection).await.unwrap();
//
//        // Load the group state again
//        let loaded_group_state = StorableDsGroupData::load(&qgid, &ds.db_connection)
//            .await
//            .unwrap();
//
//        match loaded_group_state {
//            LoadResult::Success(loaded_group_state) => {
//                assert_eq!(
//                    loaded_group_state.encrypted_group_state,
//                    storable_group_data.encrypted_group_state
//                );
//            }
//            e => {
//                panic!("Error loading group state: {:?}.", e);
//            }
//        }
//    }
//}
