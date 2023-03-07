use std::{collections::HashMap, convert::TryInto};

use chrono::{DateTime, Duration, Utc};
use mls_assist::{
    group::{Group, ProcessedAssistedMessage},
    messages::AssistedMessage,
    GroupEpoch, GroupId, LeafNodeIndex, ProcessedMessageContent,
};
use serde::{Deserialize, Serialize};
use tls_codec::{Deserialize as TlsDeserializeTrait, TlsDeserialize, TlsSerialize, TlsSize};
use tracing::instrument;

use crate::{
    crypto::{
        ear::{keys::GroupStateEarKey, EarEncryptable},
        mac::keys::MemberAuthenticationKey,
        signatures::keys::UserAuthKey,
        EncryptedDsGroupState,
    },
    messages::{
        client_ds::{AddUsersParams, AddUsersParamsAad, ClientToClientMsg},
        intra_backend::DsFanOutMessage,
    },
    qs::{QsClientReference, QsEnqueueProvider},
};

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::{MessageDistributionError, UpdateQueueConfigError, UserAdditionError},
};

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
    queue_config: QsClientReference,
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

#[derive(
    Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TlsSerialize, TlsDeserialize, TlsSize,
)]
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
    client_queue_config: QsClientReference,
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
        creator_queue_config: QsClientReference,
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
    pub(super) async fn distribute_message<Q: QsEnqueueProvider>(
        &self,
        qs_enqueue_provider: &Q,
        message: &ClientToClientMsg,
        sender_index: LeafNodeIndex,
    ) -> Result<(), MessageDistributionError> {
        for (leaf_index, client_profile) in self.client_profiles.iter() {
            if leaf_index == &sender_index {
                continue;
            }
            let client_queue_config = client_profile.client_queue_config.clone();

            let ds_fan_out_msg = DsFanOutMessage {
                payload: message.assisted_message.clone(),
                client_reference: client_queue_config,
            };

            qs_enqueue_provider
                .enqueue(ds_fan_out_msg)
                .await
                .map_err(|_| MessageDistributionError::DeliveryError)?;
        }
        Ok(())
    }

    pub(crate) fn update_queue_config(
        &mut self,
        leaf_index: LeafNodeIndex,
        client_queue_config: &QsClientReference,
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

    pub(crate) fn add_user(
        &mut self,
        params: AddUsersParams,
    ) -> Result<ClientToClientMsg, UserAdditionError> {
        // Deserialize assisted message.
        // TODO: In the future, we shouldn't have to deserialize here.
        let assisted_message: AssistedMessage = (&params.commit)
            .try_into()
            .map_err(|_| UserAdditionError::InvalidMessage)?;

        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message = if matches!(assisted_message, AssistedMessage::Commit(_)) {
            self.group()
                .process_assisted_message(assisted_message)
                .map_err(|_| UserAdditionError::ProcessingError)?
        } else {
            return Err(UserAdditionError::InvalidMessage);
        };

        // Perform DS-level validation
        // TODO: Verify that the added clients belong to one user. This requires
        // us to define the credentials we're using. To do that, we'd need to
        // modify OpenMLS.

        // Validate that the AAD includes enough encrypted credential chains
        if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
            processed_assisted_message
        {
            let aad =
                AddUsersParamsAad::tls_deserialize(&mut processed_message.authenticated_data())
                    .map_err(|_| UserAdditionError::InvalidMessage)?;
            if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
                processed_message.content()
            {
                if staged_commit.add_proposals().count()
                    != aad.encrypted_credential_information.len()
                {
                    return Err(UserAdditionError::InvalidMessage);
                }
            } else {
                return Err(UserAdditionError::InvalidMessage);
            };
        } else {
            // This should be a commit.
            return Err(UserAdditionError::InvalidMessage);
        }

        // TODO: Validate that the adder has sufficient privileges (if this
        //       isn't done by an MLS extension).

        // TODO: Validate the Welcome messages

        // TODO: Validate timestamp on key package batch.

        // TODO: Update user profiles and client profiles.

        // Everything seems to be okay.
        // Now we have to update the group state and distribute. That should
        // probably be somewhat atomic. Maybe we should even persist the message
        // alongside the encrypted group state in case something goes wrong.
        // Build a message that we can distribute.

        // For now we distribute the message first.
        let c2c_message = ClientToClientMsg {
            assisted_message: params.commit,
        };

        // Now we accept the message into the group state ...
        self.group_mut().accept_processed_message(
            processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        Ok(c2c_message)
    }
}

impl EarEncryptable<GroupStateEarKey, EncryptedDsGroupState> for DsGroupState {}
