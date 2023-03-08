use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use mls_assist::{group::Group, GroupEpoch, GroupInfo, LeafNodeIndex, Node};
use serde::{Deserialize, Serialize};
use tls_codec::{
    Deserialize as TlsDeserializeTrait, Serialize as TlsSerializeTrait, Size, TlsDeserialize,
    TlsSerialize, TlsSize,
};

use crate::{
    crypto::{
        ear::{keys::GroupStateEarKey, EarEncryptable},
        signatures::keys::{QsVerifyingKey, UserAuthKey},
        EncryptedDsGroupState,
    },
    messages::{
        client_ds::{ClientToClientMsg, UpdateQsClientReferenceParams, WelcomeInfoParams},
        intra_backend::DsFanOutMessage,
    },
    qs::{Fqdn, QsClientReference, QsEnqueueProvider},
};

use super::errors::{MessageDistributionError, UpdateQueueConfigError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeStamp {
    time: DateTime<Utc>,
}

impl Size for TimeStamp {
    fn tls_serialized_len(&self) -> usize {
        8
    }
}

impl TlsSerializeTrait for TimeStamp {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.time
            .timestamp_millis()
            .to_be_bytes()
            .tls_serialize(writer)
    }
}

impl TlsDeserializeTrait for TimeStamp {
    fn tls_deserialize<R: std::io::Read>(bytes: &mut R) -> Result<Self, tls_codec::Error>
    where
        Self: Sized,
    {
        let mut millis_bytes = [0u8; 8];
        bytes.read_exact(&mut millis_bytes)?;
        let millis = i64::from_be_bytes(millis_bytes);
        let time = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_millis(millis).ok_or(tls_codec::Error::InvalidInput)?,
            Utc,
        );
        Ok(Self { time })
    }
}

impl TimeStamp {
    pub(crate) fn now() -> Self {
        let time = Utc::now();
        Self { time }
    }

    pub(crate) fn has_expired(&self, expiration_days: i64) -> bool {
        Utc::now() - Duration::days(expiration_days) >= self.time
    }
}

#[derive(
    Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TlsSerialize, TlsDeserialize, TlsSize,
)]
pub struct UserKeyHash {
    pub(super) hash: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct UserProfile {
    // The clients associated with this user in this group
    pub(super) clients: Vec<LeafNodeIndex>,
    pub(super) user_auth_key: UserAuthKey,
}

#[derive(Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize, Clone)]
pub struct EncryptedCredentialChain {}

#[derive(Serialize, Deserialize)]
pub(super) struct ClientProfile {
    pub(super) leaf_index: LeafNodeIndex,
    pub(super) credential_chain: EncryptedCredentialChain,
    pub(super) client_queue_config: QsClientReference,
    pub(super) activity_time: TimeStamp,
    pub(super) activity_epoch: GroupEpoch,
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
    pub(super) group: Group,
    pub(super) user_profiles: HashMap<UserKeyHash, UserProfile>,
    // Here we keep users that haven't set their user key yet.
    pub(super) unmerged_users: Vec<Vec<LeafNodeIndex>>,
    pub(super) client_profiles: HashMap<LeafNodeIndex, ClientProfile>,
}

impl DsGroupState {
    //#[instrument(level = "trace", skip_all)]
    pub(crate) fn new(
        group: Group,
        creator_user_auth_key: UserAuthKey,
        creator_encrypted_credential_chain: EncryptedCredentialChain,
        creator_queue_config: QsClientReference,
    ) -> Self {
        let creator_key_hash = creator_user_auth_key.hash();
        let creator_profile = UserProfile {
            clients: vec![LeafNodeIndex::new(0u32)],
            user_auth_key: creator_user_auth_key,
        };
        let user_profiles = [(creator_key_hash, creator_profile)].into();

        let creator_client_profile = ClientProfile {
            credential_chain: creator_encrypted_credential_chain,
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

    /// Distribute the given MLS message (currently only works with ciphertexts).
    pub(super) async fn distribute_message<Q: QsEnqueueProvider>(
        &self,
        qs_enqueue_provider: &Q,
        message: ClientToClientMsg,
        sender_index: LeafNodeIndex,
    ) -> Result<(), MessageDistributionError> {
        for (leaf_index, client_profile) in self.client_profiles.iter() {
            if leaf_index == &sender_index {
                continue;
            }
            let client_queue_config = client_profile.client_queue_config.clone();

            let ds_fan_out_msg = DsFanOutMessage {
                payload: message.clone(),
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
        params: UpdateQsClientReferenceParams,
    ) -> Result<(), UpdateQueueConfigError> {
        let client_profile = self
            .client_profiles
            .get_mut(&params.sender())
            .ok_or(UpdateQueueConfigError::UnknownSender)?;
        client_profile.client_queue_config = params.new_queue_config().clone();
        Ok(())
    }

    pub(crate) fn get_user_key(&self, user_key_hash: &UserKeyHash) -> Option<&UserAuthKey> {
        self.user_profiles
            .get(user_key_hash)
            .map(|user_profile| &user_profile.user_auth_key)
    }

    /// TODO: Get the verifying key from the QS. Either look it up directly or from a local cache.
    pub(super) fn get_qs_verifying_key(&self, _fqdn: &Fqdn) -> Result<QsVerifyingKey, &str> {
        todo!()
    }

    pub(super) fn welcome_info(
        &mut self,
        welcome_info_params: WelcomeInfoParams,
    ) -> Option<&[Option<Node>]> {
        self.group_mut().past_group_state(
            &welcome_info_params.epoch,
            welcome_info_params.sender.signature_key(),
        )
    }

    pub(super) fn external_commit_info(&mut self) -> (GroupInfo, Vec<Option<Node>>) {
        let group_info = self.group().group_info().clone();
        let nodes = self.group().export_ratchet_tree();
        (group_info, nodes)
    }
}

impl EarEncryptable<GroupStateEarKey, EncryptedDsGroupState> for DsGroupState {}
