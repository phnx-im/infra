// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

use mls_assist::{
    MlsAssistRustCrypto,
    group::Group,
    openmls::{
        group::GroupId,
        prelude::{GroupEpoch, LeafNodeIndex},
        treesync::RatchetTree,
    },
    provider_traits::MlsAssistProvider,
};
use phnxtypes::{
    codec::PhnxCodec,
    crypto::{
        ear::{
            Ciphertext, EarDecryptable, EarEncryptable,
            keys::{EncryptedIdentityLinkKey, GroupStateEarKey},
        },
        errors::{DecryptionError, EncryptionError},
    },
    errors::CborMlsAssistStorage,
    identifiers::{QsReference, SealedClientReference},
    messages::client_ds::WelcomeInfoParams,
    time::TimeStamp,
};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

use crate::errors::StorageError;

use super::{GROUP_STATE_EXPIRATION, ReservedGroupId, process::ExternalCommitInfo};

pub(super) mod persistence;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct MemberProfile {
    pub(super) leaf_index: LeafNodeIndex,
    pub(super) encrypted_identity_link_key: EncryptedIdentityLinkKey,
    pub(super) client_queue_config: QsReference,
    pub(super) activity_time: TimeStamp,
    pub(super) activity_epoch: GroupEpoch,
}

/// The `DsGroupState` is the per-group state that the DS persists.
/// It is encrypted-at-rest with a roster key.
///
/// TODO: Past group states are now included in mls-assist. However, we might
/// have to store client credentials externally.
pub struct DsGroupState {
    pub(super) group: Group,
    pub(super) provider: MlsAssistRustCrypto<PhnxCodec>,
    pub(super) member_profiles: BTreeMap<LeafNodeIndex, MemberProfile>,
}

impl DsGroupState {
    pub(crate) fn new(
        provider: MlsAssistRustCrypto<PhnxCodec>,
        group: Group,
        creator_encrypted_identity_link_key: EncryptedIdentityLinkKey,
        creator_queue_config: QsReference,
    ) -> Self {
        let creator_client_profile = MemberProfile {
            encrypted_identity_link_key: creator_encrypted_identity_link_key,
            client_queue_config: creator_queue_config,
            activity_time: TimeStamp::now(),
            activity_epoch: 0u64.into(),
            leaf_index: LeafNodeIndex::new(0u32),
        };
        let client_profiles = [(LeafNodeIndex::new(0u32), creator_client_profile)].into();
        Self {
            provider,
            group,
            member_profiles: client_profiles,
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
        let encrypted_identity_link_keys = self.encrypted_identity_link_keys();
        ExternalCommitInfo {
            group_info,
            ratchet_tree,
            encrypted_identity_link_keys,
        }
    }

    /// Create a vector of encrypted identity link keys from the current list of
    /// client records.
    pub(super) fn encrypted_identity_link_keys(&self) -> Vec<EncryptedIdentityLinkKey> {
        self.member_profiles
            .values()
            .map(|client_profile| client_profile.encrypted_identity_link_key.clone())
            .collect()
    }

    pub(super) fn encrypt(
        self,
        ear_key: &GroupStateEarKey,
    ) -> Result<EncryptedDsGroupState, DsGroupStateEncryptionError> {
        let encrypted =
            EncryptableDsGroupState::from(SerializableDsGroupState::from_group_state(self)?)
                .encrypt(ear_key)?;
        Ok(encrypted)
    }

    pub(super) fn decrypt(
        encrypted_group_state: &EncryptedDsGroupState,
        ear_key: &GroupStateEarKey,
    ) -> Result<Self, DsGroupStateDecryptionError> {
        let encryptable = EncryptableDsGroupState::decrypt(ear_key, encrypted_group_state)?;
        let group_state = SerializableDsGroupState::into_group_state(encryptable.into())?;
        Ok(group_state)
    }
}

#[derive(Debug, Error)]
pub(super) enum DsGroupStateEncryptionError {
    #[error("Error decrypting group state: {0}")]
    EncryptionError(#[from] EncryptionError),
    #[error("Error deserializing group state: {0}")]
    DeserializationError(#[from] phnxtypes::codec::Error),
}

impl From<DsGroupStateEncryptionError> for tonic::Status {
    fn from(value: DsGroupStateEncryptionError) -> Self {
        match value {
            DsGroupStateEncryptionError::EncryptionError(error) => {
                error!(%error, "Failed to encrypt group state");
                Self::internal("Failed to encrypt group state")
            }
            DsGroupStateEncryptionError::DeserializationError(error) => {
                error!(%error, "Failed to deserialize group state");
                Self::internal("Failed to deserialize group state")
            }
        }
    }
}

#[derive(Debug, Error)]
pub(super) enum DsGroupStateDecryptionError {
    #[error("Error decrypting group state: {0}")]
    DecryptionError(#[from] DecryptionError),
    #[error("Error deserializing group state: {0}")]
    DeserializationError(#[from] phnxtypes::codec::Error),
}

impl From<DsGroupStateDecryptionError> for tonic::Status {
    fn from(error: DsGroupStateDecryptionError) -> Self {
        match error {
            DsGroupStateDecryptionError::DecryptionError(error) => {
                error!(%error, "Failed to decrypt group state");
                Self::internal("Failed to decrypt group state")
            }
            DsGroupStateDecryptionError::DeserializationError(error) => {
                error!(%error, "Failed to deserialize group state");
                Self::internal("Failed to deserialize group state")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct EncryptedDsGroupState(Ciphertext);

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct StorableDsGroupData {
    group_id: Uuid,
    pub(super) encrypted_group_state: EncryptedDsGroupState,
    last_used: TimeStamp,
    deleted_queues: Vec<SealedClientReference>,
}

impl StorableDsGroupData {
    pub(super) async fn new_and_store<'a>(
        connection: impl PgExecutor<'a>,
        group_id: ReservedGroupId,
        encrypted_group_state: EncryptedDsGroupState,
    ) -> Result<Self, StorageError> {
        let group_data = Self {
            group_id: group_id.0,
            encrypted_group_state,
            last_used: TimeStamp::now(),
            deleted_queues: vec![],
        };
        group_data.store(connection).await?;
        Ok(group_data)
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

#[derive(Serialize, Deserialize)]
pub(crate) struct SerializableDsGroupState {
    group_id: GroupId,
    serialized_provider: Vec<u8>,
    member_profiles: Vec<(LeafNodeIndex, MemberProfile)>,
}

impl SerializableDsGroupState {
    pub(super) fn from_group_state(
        group_state: DsGroupState,
    ) -> Result<Self, phnxtypes::codec::Error> {
        let group_id = group_state
            .group()
            .group_info()
            .group_context()
            .group_id()
            .clone();
        let client_profiles = group_state.member_profiles.into_iter().collect();
        let serialized_provider = group_state.provider.storage().serialize()?;
        Ok(Self {
            group_id,
            serialized_provider,
            member_profiles: client_profiles,
        })
    }

    pub(super) fn into_group_state(self) -> Result<DsGroupState, phnxtypes::codec::Error> {
        let storage = CborMlsAssistStorage::deserialize(&self.serialized_provider)?;
        // We unwrap here, because the constructor ensures that `self` always stores a group
        let group = Group::load(&storage, &self.group_id)?.unwrap();
        let client_profiles = self.member_profiles.into_iter().collect();
        let provider = MlsAssistRustCrypto::from(storage);
        Ok(DsGroupState {
            provider,
            group,
            member_profiles: client_profiles,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub(super) enum EncryptableDsGroupState {
    V1(SerializableDsGroupState),
}

impl From<EncryptableDsGroupState> for SerializableDsGroupState {
    fn from(encryptable: EncryptableDsGroupState) -> Self {
        match encryptable {
            EncryptableDsGroupState::V1(serializable) => serializable,
        }
    }
}

impl From<SerializableDsGroupState> for EncryptableDsGroupState {
    fn from(serializable: SerializableDsGroupState) -> Self {
        EncryptableDsGroupState::V1(serializable)
    }
}

impl EarEncryptable<GroupStateEarKey, EncryptedDsGroupState> for EncryptableDsGroupState {}
impl EarDecryptable<GroupStateEarKey, EncryptedDsGroupState> for EncryptableDsGroupState {}
