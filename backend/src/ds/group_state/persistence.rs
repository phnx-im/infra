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
use sea_orm::{DerivePrimaryKey, PrimaryKeyTrait};

use sea_orm_migration::prelude::*;
use thiserror::Error;
use tls_codec::DeserializeBytes as _;

use super::EncryptedDsGroupState;

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

impl TryFrom<Model> for EncryptedDsGroupState {
    type Error = serde_json::Error;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let ciphertext: Ciphertext = serde_json::from_slice(&model.ciphertext)?;
        let deleted_queues: Vec<SealedClientReference> =
            serde_json::from_slice(&model.deleted_queues)?;
        let last_used = TimeStamp::from(model.last_used);
        let encrypted_group_state = EncryptedDsGroupState {
            ciphertext,
            last_used,
            deleted_queues,
        };
        Ok(encrypted_group_state)
    }
}

use self::Entity as EncryptedGroupData;

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Error)]
pub(crate) enum StorageError {
    #[error(transparent)]
    DatabaseError(#[from] sea_orm::error::DbErr),
    #[error("Error deserializing group id: {0}")]
    TlsCodec(#[from] tls_codec::Error),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] serde_json::Error),
}

impl EncryptedDsGroupState {
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
}
