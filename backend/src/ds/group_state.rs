use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mls_assist::{group::Group, GroupEpoch, GroupId, LeafNodeIndex};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use tracing::instrument;

use crate::{
    crypto::{
        ear::{keys::GroupStateEarKey, EarEncryptable},
        mac::keys::MemberAuthenticationKey,
        signatures::keys::UserAuthKey,
        EncryptedDsGroupState,
    },
    messages::{client_backend::ClientToClientMsg, intra_backend::DsFanOutMessage},
    qs::{storage_provider_trait::QsStorageProvider, ClientQueueConfig, Qs, WebsocketNotifier},
};

use super::errors::{MessageDistributionError, UpdateQueueConfigError};

#[derive(Serialize, Deserialize)]
pub struct TimeStamp {
    time: Vec<u8>,
}

impl TimeStamp {
    pub(crate) fn now() -> Self {
        todo!()
    }
}

pub(crate) fn sample_group_id() -> GroupId {
    todo!()
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Role {
    Member, // Can not modify the roster beyond their own entry and even then not change their own privilege.
    Admin,  // Can to everything
}

#[derive(Serialize, Deserialize)]
struct RosterEntry {
    mac_key: MemberAuthenticationKey,
    queue_config: ClientQueueConfig,
    role: Role,
}

/// RosterDelta indicates whether a member was added, removed or updated.
/// The purpose of the `is_blank` flag is to indicate if the specified roster entry should be blanked or not.
/// When a member is added, `index` is the leaf index in the ratcheting tree
/// and `is_blank` is `false`.
/// When a member is removed, `index` is the index in the ratcheting of the member that is to be removed
/// and `is_blank` in `true`.
/// /// When a member is updated, `index` is the index in the ratcheting of the member that is to be updated
/// and `is_blank` in `false`.
#[derive(Serialize, Deserialize)]
pub struct RosterDelta {
    index: u32,
    /// TODO: This should be an enum that if a member is added, contains the `PublicRosterEntry` of the new member.
    is_blank: bool,
    role: Role,
}

#[derive(Serialize, Deserialize)]
struct Roster {
    entries: HashMap<LeafNodeIndex, RosterEntry>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserKeyHash {
    hash: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct UserProfile {
    // The clients associated with this user in this group
    clients: Vec<LeafNodeIndex>,
    user_auth_key: UserAuthKey,
}

#[derive(Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EncryptedCredentialChain {}

#[derive(Serialize, Deserialize)]
pub(super) struct ClientProfile {
    credential_chain: EncryptedCredentialChain,
    client_queue_config: ClientQueueConfig,
    activity_time: DateTime<Utc>,
    activity_epoch: GroupEpoch,
}

#[derive(Serialize, Deserialize)]
pub(super) struct ProposalStore {}

/// The `DsGroupState` is the per-group state that the DS persists.
/// It is encrypted-at-rest with a roster key.
///
/// TODO: Past group states are now included in mls-assist. However, we might
/// have to store client credentials externally.
#[derive(Serialize, Deserialize)]
pub(crate) struct DsGroupState {
    group: Group,
    user_profiles: HashMap<UserKeyHash, UserProfile>,
    client_profiles: HashMap<LeafNodeIndex, ClientProfile>,
}

impl DsGroupState {
    //#[instrument(level = "trace", skip_all)]
    pub(crate) fn new(
        group: Group,
        creator_user_auth_key: UserAuthKey,
        creator_encrypted_credential_chain: EncryptedCredentialChain,
        creator_queue_config: ClientQueueConfig,
    ) -> Self {
        let creator_profile = UserProfile {
            clients: vec![LeafNodeIndex::new(0u32)],
            user_auth_key: creator_user_auth_key,
        };
        let creator_key_hash = creator_profile.user_auth_key.hash();
        let user_profiles = [(creator_key_hash, creator_profile)].into();

        let creator_client_profile = ClientProfile {
            credential_chain: creator_encrypted_credential_chain,
            client_queue_config: creator_queue_config,
            activity_time: Utc::now(),
            activity_epoch: 0u64.into(),
        };
        let client_profiles = [(LeafNodeIndex::new(0u32), creator_client_profile)].into();
        Self {
            group,
            user_profiles,
            client_profiles,
        }
    }

    /// Get a reference to the public group state.
    pub(crate) fn group(&self) -> &Group {
        &self.group
    }

    /// Get a mutable reference to the public group state.
    pub(crate) fn group_mut(&mut self) -> &mut Group {
        &mut self.group
    }

    /// Check if the given role has the permission to apply the given roster deltas.
    pub(crate) fn check_privileges(&self, _sender: &LeafNodeIndex, _roster_deltas: &[RosterDelta]) {
        todo!()
    }

    /// Distribute the given MLS message (currently only works with ciphertexts).
    pub(super) async fn distribute_message<W: WebsocketNotifier, Qsp: QsStorageProvider>(
        &self,
        qs_handle: &Qsp,
        websocket_notifier: &W,
        message: &ClientToClientMsg,
    ) -> Result<(), MessageDistributionError> {
        for (leaf_index, client_profile) in self.client_profiles.iter() {
            if leaf_index == &message.sender() {
                continue;
            }
            let client_queue_config = client_profile.client_queue_config.clone();

            let ds_fan_out_msg = DsFanOutMessage {
                payload: message.clone(),
                queue_config: client_queue_config,
            };

            Qs::enqueue_message(qs_handle, websocket_notifier, ds_fan_out_msg)
                .await
                .map_err(|_| MessageDistributionError::DeliveryError)?;
        }
        Ok(())
    }

    pub(crate) fn update_queue_config(
        &mut self,
        leaf_index: LeafNodeIndex,
        client_queue_config: &ClientQueueConfig,
    ) -> Result<(), UpdateQueueConfigError> {
        let client_profile = self
            .client_profiles
            .get_mut(&leaf_index)
            .ok_or(UpdateQueueConfigError::UnknownSender)?;
        client_profile.client_queue_config = client_queue_config.clone();
        Ok(())
    }

    pub(crate) fn get_user_key(&self, user_key_hash: &UserKeyHash) -> Option<&UserAuthKey> {
        self.user_profiles
            .get(user_key_hash)
            .map(|user_profile| &user_profile.user_auth_key)
    }

    // Result
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn apply_roster_deltas(&mut self, _roster_deltas: &[RosterDelta]) {
        todo!()
    }
}

impl EarEncryptable<GroupStateEarKey, EncryptedDsGroupState> for DsGroupState {}
