// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::codec::PhnxCodec;
use phnxtypes::identifiers::{QualifiedGroupId, SealedClientReference};
use phnxtypes::time::TimeStamp;
use sea_orm::entity::prelude::{
    ActiveModelBehavior, DbConn, DeriveEntityModel, DeriveRelation, EntityTrait, EnumIter, Uuid,
};
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ActiveModelTrait, DerivePrimaryKey, PrimaryKeyTrait};

use sea_orm_migration::prelude::*;
use thiserror::Error;

use crate::ds::GROUP_STATE_EXPIRATION;

use super::{EncryptedDsGroupState, ReservedGroupId, StorableDsGroupData};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "encrypted_group_data")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub group_id: Uuid,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub encrypted_group_state: Vec<u8>,
    pub last_used: DateTimeUtc,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub deleted_queues: Vec<u8>,
}

impl TryFrom<Model> for StorableDsGroupData {
    type Error = phnxtypes::codec::Error;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let ciphertext: EncryptedDsGroupState =
            PhnxCodec::from_slice(&model.encrypted_group_state)?;
        let deleted_queues: Vec<SealedClientReference> =
            PhnxCodec::from_slice(&model.deleted_queues)?;
        let last_used = TimeStamp::from(model.last_used);
        let encrypted_group_state = StorableDsGroupData {
            group_id: model.group_id,
            encrypted_group_state: ciphertext,
            last_used,
            deleted_queues,
        };
        Ok(encrypted_group_state)
    }
}

pub(crate) use self::Entity as EncryptedGroupData;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    DatabaseError(#[from] sea_orm::error::DbErr),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxtypes::codec::Error),
}

/// Return value of a group state load query.
#[derive(Debug)]
pub(crate) enum LoadResult {
    Success(StorableDsGroupData),
    // Reserved indicates that the group id was reserved at the given time
    // stamp.
    Reserved(ReservedGroupId),
    NotFound,
    Expired,
}

impl StorableDsGroupData {
    pub(super) async fn store(&self, connection: &DbConn) -> Result<(), StorageError> {
        let model = Model {
            group_id: self.group_id,
            encrypted_group_state: PhnxCodec::to_vec(&self.encrypted_group_state)?,
            last_used: self.last_used.into(),
            deleted_queues: PhnxCodec::to_vec(&self.deleted_queues)?,
        };
        let active_model = ActiveModel::from(model).reset_all();
        let on_conflict_behaviour = OnConflict::column(Column::GroupId).do_nothing().to_owned();
        EncryptedGroupData::insert(active_model)
            .on_conflict(on_conflict_behaviour)
            .exec(connection)
            .await?;
        Ok(())
    }

    pub(crate) async fn load(
        qgid: &QualifiedGroupId,
        connection: &DbConn,
    ) -> Result<LoadResult, StorageError> {
        let Some(model) = EncryptedGroupData::find_by_id(qgid.group_uuid())
            .one(connection)
            .await?
        else {
            return Ok(LoadResult::NotFound);
        };
        let group_data = Self::try_from(model)?;

        if group_data.last_used.has_expired(GROUP_STATE_EXPIRATION) {
            return Ok(LoadResult::Expired);
        }

        if group_data.encrypted_group_state == EncryptedDsGroupState::default() {
            return Ok(LoadResult::Reserved(ReservedGroupId(group_data.group_id)));
        }

        Ok(LoadResult::Success(group_data))
    }

    pub(crate) async fn update(&self, connection: &DbConn) -> Result<(), StorageError> {
        let model = Model {
            group_id: self.group_id,
            encrypted_group_state: PhnxCodec::to_vec(&self.encrypted_group_state)?,
            last_used: self.last_used.into(),
            deleted_queues: PhnxCodec::to_vec(&self.deleted_queues)?,
        };
        let active_model = ActiveModel::from(model).reset_all();
        active_model.update(connection).await?;
        Ok(())
    }

    pub(crate) async fn _delete(
        qgid: &QualifiedGroupId,
        connection: &DbConn,
    ) -> Result<(), StorageError> {
        ActiveModel {
            group_id: sea_orm::ActiveValue::Set(qgid.group_uuid()),
            ..Default::default()
        }
        .delete(connection)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use phnxtypes::{
        crypto::ear::Ciphertext,
        identifiers::{Fqdn, QualifiedGroupId},
    };
    use uuid::Uuid;

    use crate::ds::{
        group_state::{
            persistence::LoadResult, EncryptedDsGroupState, ReservedGroupId, StorableDsGroupData,
        },
        Ds,
    };

    #[tokio::test]
    async fn reserve_group_id() {
        let ds = Ds::new_ephemeral(Fqdn::try_from("example.com").unwrap())
            .await
            .expect("Error creating ephemeral Ds instance.");

        // Sample a random group id and reserve it
        let group_uuid = Uuid::new_v4();
        let was_reserved = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
            .await
            .unwrap();
        assert!(was_reserved);

        // Try to reserve the same group id again
        let was_reserved_again = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
            .await
            .expect("Error reserving group id.");

        // This should return false
        assert!(!was_reserved_again);
    }

    #[tokio::test]
    async fn group_state_lifecycle() {
        let ds = Ds::new_ephemeral(Fqdn::try_from("example.com").unwrap())
            .await
            .expect("Error creating ephemeral Ds instance.");

        let dummy_ciphertext = Ciphertext::dummy();
        let test_state: EncryptedDsGroupState = dummy_ciphertext.into();

        // Create/store a dummy group state
        let group_uuid = Uuid::new_v4();
        let was_reserved = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
            .await
            .unwrap();
        assert!(was_reserved);

        // Load the reserved group id
        let qgid = QualifiedGroupId::new(group_uuid, ds.own_domain.clone());
        let LoadResult::Reserved(reserved_group_id) =
            StorableDsGroupData::load(&qgid, &ds.db_connection)
                .await
                .unwrap()
        else {
            panic!("Error loading group state.");
        };

        let mut storable_group_data =
            StorableDsGroupData::new(reserved_group_id, test_state.clone());

        // Save the group state
        storable_group_data
            .update(&ds.db_connection)
            .await
            .expect("Error saving group state.");

        // Load the group state again
        let loaded_group_state = StorableDsGroupData::load(&qgid, &ds.db_connection)
            .await
            .unwrap();

        if let LoadResult::Success(loaded_group_state) = loaded_group_state {
            assert_eq!(
                loaded_group_state.encrypted_group_state,
                storable_group_data.encrypted_group_state
            );
        } else {
            panic!("Error loading group state.");
        }

        // Try to reserve the group id of the created group state
        let successfully_reserved = ReservedGroupId::reserve(&ds.db_connection, group_uuid)
            .await
            .unwrap();

        // This should return false
        assert!(!successfully_reserved);

        // Update that group state.
        storable_group_data.encrypted_group_state.0.flip_bit();

        storable_group_data.update(&ds.db_connection).await.unwrap();

        // Load the group state again
        let loaded_group_state = StorableDsGroupData::load(&qgid, &ds.db_connection)
            .await
            .unwrap();

        match loaded_group_state {
            LoadResult::Success(loaded_group_state) => {
                assert_eq!(
                    loaded_group_state.encrypted_group_state,
                    storable_group_data.encrypted_group_state
                );
            }
            e => {
                panic!("Error loading group state: {:?}.", e);
            }
        }
    }
}
