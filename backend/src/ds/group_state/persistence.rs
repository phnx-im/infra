// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::group::GroupId;
use phnxtypes::crypto::ear::Ciphertext;
use phnxtypes::identifiers::{QualifiedGroupId, SealedClientReference};
use phnxtypes::time::TimeStamp;
use sea_orm::entity::prelude::{
    ActiveModelBehavior, DbConn, DeriveEntityModel, DeriveRelation, EntityTrait, EnumIter, Uuid,
};
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ActiveModelTrait, DerivePrimaryKey, PrimaryKeyTrait};

use sea_orm_migration::prelude::*;
use thiserror::Error;
use tls_codec::DeserializeBytes as _;

use super::StorableDsGroupData;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "encrypted_groups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub group_id: Uuid,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub ciphertext: Vec<u8>,
    pub last_used: DateTimeUtc,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub deleted_queues: Vec<u8>,
}

impl TryFrom<Model> for StorableDsGroupData {
    type Error = serde_json::Error;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let ciphertext: Ciphertext = serde_json::from_slice(&model.ciphertext)?;
        let deleted_queues: Vec<SealedClientReference> =
            serde_json::from_slice(&model.deleted_queues)?;
        let last_used = TimeStamp::from(model.last_used);
        let encrypted_group_state = StorableDsGroupData {
            group_id: model.group_id,
            ciphertext: ciphertext.into(),
            last_used,
            deleted_queues,
        };
        Ok(encrypted_group_state)
    }
}

impl TryFrom<StorableDsGroupData> for Model {
    type Error = serde_json::Error;

    fn try_from(storable_group_state: StorableDsGroupData) -> Result<Self, Self::Error> {
        let ciphertext = serde_json::to_vec(&storable_group_state.ciphertext)?;
        let deleted_queues = serde_json::to_vec(&storable_group_state.deleted_queues)?;
        let last_used = storable_group_state.last_used.into();
        let model = Model {
            group_id: storable_group_state.group_id,
            ciphertext,
            last_used,
            deleted_queues,
        };
        Ok(model)
    }
}

use self::Entity as EncryptedGroupData;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub(crate) trait Persistable: Sized {
    type PrimaryKey;

    fn store(&self, group_id: &GroupId, connection: &DbConn) -> Result<(), StorageError>;
    fn load(group_id: &GroupId, connection: &DbConn) -> Result<Option<Self>, StorageError>;
    fn update(&self, group_id: &GroupId, connection: &DbConn) -> Result<(), StorageError>;
    fn delete(group_id: &GroupId, connection: &DbConn) -> Result<(), StorageError>;
}

#[derive(Debug, Error)]
pub(crate) enum StorageError {
    #[error(transparent)]
    DatabaseError(#[from] sea_orm::error::DbErr),
    #[error("Error deserializing group id: {0}")]
    TlsCodec(#[from] tls_codec::Error),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] serde_json::Error),
}

impl StorableDsGroupData {
    pub(crate) async fn store(&self, connection: &DbConn) -> Result<(), StorageError> {
        let model = Model {
            group_id: self.group_id,
            ciphertext: serde_json::to_vec(&self.ciphertext)?,
            last_used: self.last_used.into(),
            deleted_queues: serde_json::to_vec(&self.deleted_queues)?,
        };
        let active_model = ActiveModel::from(model).reset_all();
        active_model.insert(connection).await?;
        Ok(())
    }

    pub(crate) async fn load(
        group_id: &GroupId,
        connection: &DbConn,
    ) -> Result<Option<Self>, StorageError> {
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(group_id.as_slice())?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);
        let Some(model) = EncryptedGroupData::find_by_id(group_uuid)
            .one(connection)
            .await?
        else {
            return Ok(None);
        };
        let result = Self::try_from(model)?;

        Ok(Some(result))
    }

    pub(crate) async fn update(&self, connection: &DbConn) -> Result<(), StorageError> {
        let model = Model {
            group_id: self.group_id,
            ciphertext: serde_json::to_vec(&self.ciphertext)?,
            last_used: self.last_used.into(),
            deleted_queues: serde_json::to_vec(&self.deleted_queues)?,
        };
        let active_model = ActiveModel::from(model).reset_all();
        active_model.update(connection).await?;
        Ok(())
    }

    pub(crate) async fn delete(
        group_id: &GroupId,
        connection: &DbConn,
    ) -> Result<(), StorageError> {
        let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(group_id.as_slice())?;
        let group_uuid = Uuid::from_bytes(qgid.group_id);
        ActiveModel {
            group_id: sea_orm::ActiveValue::Set(group_uuid),
            ..Default::default()
        }
        .delete(connection)
        .await?;
        Ok(())
    }
}
