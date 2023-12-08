// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Debug;

use async_trait::async_trait;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedGroupId},
    time::TimeStamp,
};
use uuid::Uuid;

use self::group_state::EncryptedDsGroupState;

mod add_clients;
mod add_users;
pub mod api;
mod delete_group;
pub mod group_state;
mod join_connection_group;
mod join_group;
mod remove_clients;
mod remove_users;
mod resync_client;
mod self_remove_client;
mod update_client;

/// Number of days after its last use upon which a group state is considered
/// expired.
pub const GROUP_STATE_EXPIRATION_DAYS: i64 = 90;

/// Return value of a group state load query.
/// #[derive(Serialize, Deserialize)]
pub enum GenericLoadState<GroupCiphertext> {
    Success(GroupCiphertext),
    // Reserved indicates that the group id was reserved at the given time
    // stamp.
    Reserved(TimeStamp),
    NotFound,
    Expired,
}

/// Storage provider trait for the DS.
#[async_trait]
pub trait DsGenericStorageProvider<GroupCiphertext, GroupCiphertextId>:
    Sync + Send + 'static
where
    GroupCiphertext: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug + Sync + Send,
    GroupCiphertextId: AsRef<[u8; 16]> + std::fmt::Debug + Sync + Send,
{
    type StorageError: Debug + ToString;

    /// Loads the ds group state with the group ID.
    async fn load_group_state(
        &self,
        group_id: &GroupCiphertextId,
    ) -> Result<GenericLoadState<GroupCiphertext>, Self::StorageError>;

    /// Saves the ds group state with the group ID.
    async fn save_group_state(
        &self,
        group_id: &GroupCiphertextId,
        encrypted_group_state: GroupCiphertext,
    ) -> Result<(), Self::StorageError>
    where
        GroupCiphertext: 'async_trait;

    /// Reserves the ds group state slot with the given group ID.
    ///
    /// Returns false if the group ID is already taken and true otherwise.
    async fn reserve_group_id(
        &self,
        group_id: &GroupCiphertextId,
    ) -> Result<bool, Self::StorageError>;

    /// Returns the domain of this DS.
    async fn own_domain(&self) -> Fqdn;
}

/// Trait alias to make code more readable.
#[async_trait]
pub trait DsStorageProvider:
    DsGenericStorageProvider<EncryptedDsGroupState, QualifiedGroupId>
{
    //     /// Loads the ds group state with the group ID.
    // async fn load_group_state(
    // &self,
    // group_id: &QualifiedGroupId,
    // ) -> Result<LoadState, Self::StorageError> {
    // DsGenericStorageProvider::load_group_state(self, group_id).await
    // }

    // /// Saves the ds group state with the group ID.
    // async fn save_group_state(
    // &self,
    // group_id: &QualifiedGroupId,
    // encrypted_group_state: EncryptedDsGroupState,
    // ) -> Result<(), Self::StorageError> {
    // DsGenericStorageProvider::save_group_state(self, group_id, encrypted_group_state).await
    // }

    // /// Reserves the ds group state slot with the given group ID.
    // ///
    // /// Returns false if the group ID is already taken and true otherwise.
    // async fn reserve_group_id(
    // &self,
    // group_id: &QualifiedGroupId,
    // ) -> Result<bool, Self::StorageError> {
    // DsGenericStorageProvider::reserve_group_id(self, group_id).await
    // }

    // /// Returns the domain of this DS.
    // async fn own_domain(&self) -> Fqdn {
    // DsGenericStorageProvider::own_domain(self).await
    // }
}

pub type LoadState = GenericLoadState<EncryptedDsGroupState>;

#[derive(Default)]
pub struct Ds {}

impl Ds {
    /// Create a new ds instance.
    pub fn new() -> Self {
        Self {}
    }
}
