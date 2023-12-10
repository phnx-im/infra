// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Debug;

use async_trait::async_trait;
use mls_assist::openmls::prelude::GroupId;
use phnxtypes::{identifiers::Fqdn, time::TimeStamp};

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
pub enum LoadState {
    Success(EncryptedDsGroupState),
    // Reserved indicates that the group id was reserved at the given time
    // stamp.
    Reserved(TimeStamp),
    NotFound,
    Expired,
}

/// Storage provider trait for the DS.
#[async_trait]
pub trait DsStorageProvider: Sync + Send + 'static {
    type StorageError: Debug + ToString;

    /// Loads the ds group state with the group ID.
    async fn load_group_state(&self, group_id: &GroupId) -> Result<LoadState, Self::StorageError>;

    /// Saves the ds group state with the group ID.
    async fn save_group_state(
        &self,
        group_id: &GroupId,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<(), Self::StorageError>;

    /// Reserves the ds group state slot with the given group ID.
    ///
    /// Returns false if the group ID is already taken and true otherwise.
    async fn reserve_group_id(&self, group_id: &GroupId) -> Result<bool, Self::StorageError>;

    /// Returns the domain of this DS.
    async fn own_domain(&self) -> Fqdn;
}

#[derive(Default)]
pub struct Ds {}

impl Ds {
    /// Create a new ds instance.
    pub fn new() -> Self {
        Self {}
    }
}
