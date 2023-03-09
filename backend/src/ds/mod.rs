use std::fmt::Debug;

use async_trait::async_trait;
use mls_assist::GroupId;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::crypto::{ear::keys::GroupStateEarKey, signatures::signable::Signature, *};

use self::group_state::TimeStamp;

mod add_users;
pub mod api;
pub mod errors;
pub mod group_state;
mod join_group;
mod remove_users;
mod update_client;

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

/// This is the client's actual client id, not a pseudonym.
#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize, ToSchema)]
pub struct ClientId {}

#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WelcomeAttributionInfoPayload {
    sender_client_id: ClientId,
    group_credential_encryption_key: GroupStateEarKey,
}

#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WelcomeAttributionInfoTbs {
    payload: WelcomeAttributionInfoPayload,
    group_id: GroupId,
    welcome: Vec<u8>,
}

#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WelcomeAttributionInfo {
    payload: WelcomeAttributionInfoPayload,
    signature: Signature,
}

/// Storage provider trait for the DS.
#[async_trait]
pub trait DsStorageProvider: Sync + Send + 'static {
    type StorageError: Debug + ToString;

    /// Creates a new ds group state with the ciphertext. Returns the group ID.
    async fn create_group_state(
        &self,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<GroupId, Self::StorageError>;

    /// Loads the ds group state with the group ID.
    async fn load_group_state(&self, group_id: &GroupId) -> LoadState;

    /// Saves the ds group state with the group ID.
    async fn save_group_state(
        &self,
        group_id: &GroupId,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<(), Self::StorageError>;

    /// Reserves the ds group state slot with the given group ID.
    ///
    /// Returns an error if the group ID is already taken.
    async fn reserve_group_id(&self, group_id: &GroupId) -> Result<(), Self::StorageError>;
}

#[derive(Default)]
pub struct Ds {}

impl Ds {
    /// Create a new ds instance.
    pub fn new() -> Self {
        Self {}
    }

    /// Delete encrypted group states of which the time stamps have expired.
    fn clean_up_stale_groups(&mut self) {
        todo!()
    }
}
