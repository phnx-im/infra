// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, HashMap, HashSet};

use mls_assist::{
    group::{errors::StorageError, Group},
    openmls::{
        group::GroupId,
        prelude::{GroupEpoch, LeafNodeIndex, QueuedRemoveProposal, Sender},
        treesync::RatchetTree,
    },
};
use phnxtypes::{
    credentials::EncryptedClientCredential,
    crypto::{
        ear::{
            keys::{EncryptedSignatureEarKey, GroupStateEarKey},
            Ciphertext, EarDecryptable, EarEncryptable,
        },
        signatures::keys::{UserAuthVerifyingKey, UserKeyHash},
    },
    errors::{CborMlsAssistStorage, UpdateQueueConfigError, ValidationError},
    identifiers::{QsClientReference, SealedClientReference},
    messages::client_ds::{UpdateQsClientReferenceParams, WelcomeInfoParams},
    time::TimeStamp,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::{process::ExternalCommitInfo, ReservedGroupId, GROUP_STATE_EXPIRATION};

pub(super) mod persistence;

#[derive(Serialize, Deserialize)]
pub(super) struct UserProfile {
    // The clients associated with this user in this group
    pub(super) clients: Vec<LeafNodeIndex>,
    pub(super) user_auth_key: UserAuthVerifyingKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ClientProfile {
    pub(super) leaf_index: LeafNodeIndex,
    pub(super) encrypted_client_information: (EncryptedClientCredential, EncryptedSignatureEarKey),
    pub(super) client_queue_config: QsClientReference,
    pub(super) activity_time: TimeStamp,
    pub(super) activity_epoch: GroupEpoch,
}

#[derive(Serialize, Deserialize)]
pub(super) struct ProposalStore {}

#[derive(Serialize, Deserialize)]
pub(crate) struct SerializableDsGroupState {
    pub(super) group_id: GroupId,
    pub(super) serialized_provider: Vec<u8>,
    pub(super) user_profiles: Vec<(UserKeyHash, UserProfile)>,
    pub(super) unmerged_users: Vec<Vec<LeafNodeIndex>>,
    pub(super) client_profiles: Vec<(LeafNodeIndex, ClientProfile)>,
}

#[derive(Debug, Error)]
pub(super) enum SerializedGroupStateError {
    #[error("Group not found")]
    GroupNotFound,
    #[error("Storage error: {0}")]
    StorageError(StorageError<CborMlsAssistStorage>),
}

impl SerializableDsGroupState {
    pub(super) fn from_group_and_provider(
        group_state: DsGroupState,
        provider: &CborMlsAssistStorage,
    ) -> Result<Self, StorageError<CborMlsAssistStorage>> {
        let group_id = group_state
            .group()
            .group_info()
            .group_context()
            .group_id()
            .clone();
        let user_profiles = group_state.user_profiles.into_iter().collect();
        let client_profiles = group_state.client_profiles.into_iter().collect();
        let serialized_provider = provider.serialize()?;
        Ok(Self {
            group_id,
            serialized_provider,
            user_profiles,
            unmerged_users: group_state.unmerged_users,
            client_profiles,
        })
    }

    pub(super) fn into_group_state_and_provider(
        self,
    ) -> Result<(DsGroupState, CborMlsAssistStorage), SerializedGroupStateError> {
        let provider = CborMlsAssistStorage::deserialize(&self.serialized_provider)
            .map_err(SerializedGroupStateError::StorageError)?;
        let Some(group) = Group::load(&provider, &self.group_id)
            .map_err(SerializedGroupStateError::StorageError)?
        else {
            return Err(SerializedGroupStateError::GroupNotFound);
        };
        let user_profiles = self.user_profiles.into_iter().collect();
        let client_profiles = self.client_profiles.into_iter().collect();
        Ok((
            DsGroupState {
                group,
                user_profiles,
                unmerged_users: self.unmerged_users,
                client_profiles,
            },
            provider,
        ))
    }
}

/// The `DsGroupState` is the per-group state that the DS persists.
/// It is encrypted-at-rest with a roster key.
///
/// TODO: Past group states are now included in mls-assist. However, we might
/// have to store client credentials externally.
pub(crate) struct DsGroupState {
    pub(super) group: Group,
    pub(super) user_profiles: HashMap<UserKeyHash, UserProfile>,
    // Here we keep users that haven't set their user key yet.
    pub(super) unmerged_users: Vec<Vec<LeafNodeIndex>>,
    pub(super) client_profiles: BTreeMap<LeafNodeIndex, ClientProfile>,
}

impl DsGroupState {
    //#[instrument(level = "trace", skip_all)]
    pub(crate) fn new(
        group: Group,
        creator_user_auth_key: UserAuthVerifyingKey,
        creator_encrypted_client_credential: EncryptedClientCredential,
        creator_encrypted_signature_ear_key: EncryptedSignatureEarKey,
        creator_queue_config: QsClientReference,
    ) -> Self {
        let creator_key_hash = creator_user_auth_key.hash();
        let creator_profile = UserProfile {
            clients: vec![LeafNodeIndex::new(0u32)],
            user_auth_key: creator_user_auth_key,
        };
        let user_profiles = [(creator_key_hash, creator_profile)].into();

        let creator_client_profile = ClientProfile {
            encrypted_client_information: (
                creator_encrypted_client_credential,
                creator_encrypted_signature_ear_key,
            ),
            client_queue_config: creator_queue_config,
            activity_time: TimeStamp::now(),
            activity_epoch: 0u64.into(),
            leaf_index: LeafNodeIndex::new(0u32),
        };
        let client_profiles = [(LeafNodeIndex::new(0u32), creator_client_profile)].into();
        Self {
            group,
            user_profiles,
            client_profiles,
            unmerged_users: vec![],
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

    pub(crate) fn update_queue_config(
        &mut self,
        params: UpdateQsClientReferenceParams,
    ) -> Result<(), UpdateQueueConfigError> {
        let client_profile = self
            .client_profiles
            .get_mut(&params.sender())
            .ok_or(UpdateQueueConfigError::UnknownSender)?;
        client_profile.client_queue_config = params.new_queue_config().clone();
        Ok(())
    }

    pub(crate) fn get_user_key(
        &self,
        user_key_hash: &UserKeyHash,
    ) -> Option<&UserAuthVerifyingKey> {
        self.user_profiles
            .get(user_key_hash)
            .map(|user_profile| &user_profile.user_auth_key)
    }

    pub(super) fn welcome_info(
        &mut self,
        welcome_info_params: WelcomeInfoParams,
    ) -> Option<&RatchetTree> {
        self.group_mut()
            .past_group_state(&welcome_info_params.epoch, &welcome_info_params.sender)
    }

    pub(super) fn external_commit_info(&self) -> ExternalCommitInfo {
        let group_info = self.group().group_info().clone();
        let ratchet_tree = self.group().export_ratchet_tree();
        let encrypted_client_info = self.client_information();
        ExternalCommitInfo {
            group_info,
            ratchet_tree,
            encrypted_client_info,
        }
    }

    pub(super) fn process_referenced_remove_proposals(
        &mut self,
        remove_proposals: &[QueuedRemoveProposal],
    ) -> Result<(), ValidationError> {
        // Verify that we're only committing correct proposals.
        // Remove proposals (typically not allowed in the context of this endpoint)
        // Rules:
        // * If a client only removes itself, that's valid
        let mut marked_users: HashSet<UserKeyHash> = HashSet::new();
        for remove_proposal in remove_proposals {
            // For now, we only allow member proposals.
            let sender = if let Sender::Member(sender_index) = remove_proposal.sender() {
                *sender_index
            } else {
                return Err(ValidationError::InvalidMessage);
            };
            let removed = remove_proposal.remove_proposal().removed();
            if sender == removed {
                // This is valid, but we should record the affected user if it's the
                // user's only client s.t. we know to remove the user profile later.
                if let Some(user_key_hash) =
                    self.user_profiles
                        .iter()
                        .find_map(|(user_key_hash, user_profile)| {
                            if let Some(_client_index) = user_profile.clients.first() {
                                if user_profile.clients.len() == 1 {
                                    Some(user_key_hash)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                {
                    marked_users.insert(user_key_hash.clone());
                } else {
                    return Err(ValidationError::InvalidMessage);
                }
            } else {
                // Non-self-referencing remove proposals are invalid for now.
                return Err(ValidationError::InvalidMessage);
            }
        }
        // We now know that all removes are clients removing
        // themselves.
        let removed_clients: HashSet<LeafNodeIndex> = remove_proposals
            .iter()
            .map(|proposal| proposal.remove_proposal().removed())
            .collect();
        for removed_client in removed_clients {
            let removed_client_profile_option = self.client_profiles.remove(&removed_client);
            debug_assert!(removed_client_profile_option.is_some())
        }

        // Finally, we remove the client and user profiles.
        for marked_user in marked_users {
            let removed_user_profile_option = self.user_profiles.remove(&marked_user);
            debug_assert!(removed_user_profile_option.is_some())
        }
        Ok(())
    }

    /// Create vector of encrypted client credentials options from the current
    /// list of client records.
    pub(super) fn client_information(
        &self,
    ) -> Vec<(EncryptedClientCredential, EncryptedSignatureEarKey)> {
        let mut client_information = vec![];
        for (_client_index, client_profile) in self.client_profiles.iter() {
            client_information.push(client_profile.encrypted_client_information.clone());
        }
        client_information
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(transparent)]
pub struct EncryptedDsGroupState(Ciphertext);

#[derive(Debug)]
pub(super) struct StorableDsGroupData {
    group_id: Uuid,
    pub(super) encrypted_group_state: EncryptedDsGroupState,
    last_used: TimeStamp,
    deleted_queues: Vec<SealedClientReference>,
}

impl StorableDsGroupData {
    pub(super) fn new(
        group_id: ReservedGroupId,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Self {
        Self {
            group_id: group_id.0,
            encrypted_group_state,
            last_used: TimeStamp::now(),
            deleted_queues: vec![],
        }
    }

    pub(super) fn has_expired(&self) -> bool {
        self.last_used.has_expired(GROUP_STATE_EXPIRATION)
    }
}

impl From<Ciphertext> for EncryptedDsGroupState {
    fn from(ciphertext: Ciphertext) -> Self {
        Self(ciphertext)
    }
}

impl AsRef<Ciphertext> for EncryptedDsGroupState {
    fn as_ref(&self) -> &Ciphertext {
        &self.0
    }
}

impl EarEncryptable<GroupStateEarKey, EncryptedDsGroupState> for SerializableDsGroupState {}
impl EarDecryptable<GroupStateEarKey, EncryptedDsGroupState> for SerializableDsGroupState {}
