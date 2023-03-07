//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{messages::SerializedAssistedMessage, GroupId, LeafNodeIndex, SignaturePublicKey};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::keys::DeleteAuthKeyEarKey,
        kdf::keys::RosterKdfKey,
        mac::{
            keys::{EnqueueAuthKeyCtxt, QueueDeletionAuthKey},
            MacTag, TagVerifiable, TagVerified, TaggedStruct,
        },
        signatures::keys::QueueOwnerVerifyingKey,
        signatures::signable::Signature,
        RatchetPublicKey,
    },
    qs::{fanout_queue::FanOutQueueInfo, ClientQueueConfig, EncryptedPushToken, QueueId},
};

use super::{intra_backend::DsFanOutMessage, AddPackage, FriendshipToken, QsCid, QsUid};

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsFetchMessagesParams {
    pub payload: QsFetchMessageParamsTBS,
    pub signature: Signature, // A signature over the whole request using the queue owner's private key.
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsFetchMessageParamsTBS {
    pub queue_id: QueueId,          // The target queue id.
    pub sequence_number_start: u64, // The sequence number of the first message we want to fetch.
    pub max_messages: u64, // The maximum number of messages we'd like to retrieve from the QS.
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsFetchMessagesResponse {
    pub messages: Vec<EnqueuedMessage>,
    pub remaining_messages: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsDeleteQueueRequest {
    pub payload: QsDeleteQueueParams,
    pub request_hash: Vec<u8>,
    pub mac: MacTag, // A tag over the request hash using the queue's delete auth key.
}

impl TagVerifiable for QsDeleteQueueRequest {
    type VerifiedOutput = QsDeleteQueueParams;

    type Key = QueueDeletionAuthKey;

    fn payload(&self) -> &[u8] {
        &self.request_hash
    }

    fn tag(&self) -> &crate::crypto::mac::MacTag {
        &self.mac
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsDeleteQueueParams {
    pub queue_id: QueueId,
    pub auth_token_key: DeleteAuthKeyEarKey, // EAR key to decrypt the deletion auth key
}

impl TagVerified<QsDeleteQueueRequest> for QsDeleteQueueParams {
    type SealingType = private_mod::Seal;

    fn from_payload(_seal: Self::SealingType, payload: QsDeleteQueueRequest) -> Self {
        Self {
            queue_id: payload.payload.queue_id,
            auth_token_key: payload.payload.auth_token_key,
        }
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsUpdateQueueInfoParams {
    pub payload: QsUpdateQueueInfoParamsTBS,
    pub signature: Signature,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsUpdateQueueInfoParamsTBS {
    pub queue_id: QueueId,
    pub info_update: QsFanOutQueueUpdate,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsCreateQueueParams {
    pub payload: QsCreateQueueParamsTBM,
    pub signature: Signature,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsCreateQueueParamsTBM {
    pub queue_id: QueueId,
    pub queue_info: FanOutQueueInfo,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsQueueUpdate {
    pub owner_public_key_option: Option<RatchetPublicKey>,
    pub owner_signature_key_option: Option<QueueOwnerVerifyingKey>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsFanOutQueueUpdate {
    pub qs_basic_queue_update: QsQueueUpdate,
    pub encrypted_push_token_option: Option<Option<EncryptedPushToken>>,
    pub encrypted_auth_key_option: Option<EnqueueAuthKeyCtxt>,
}

pub type QsInputMessage = DsFanOutMessage;

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EnqueuedMessage {
    pub sequence_number: u64,
    pub ciphertext: Vec<u8>,
}

/// Error struct for deserialization of an [`UnverifiedGroupOperationParams`]
/// struct.
pub enum GroupOpsDeserializationError {
    DeserializationError,
    WrongRequestType,
}

// === QS ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordParams {
    user_record_auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateUserRecordParams {
    qs_uid: QsUid,
    user_record_auth_key: SignaturePublicKey,
    friendship_token: FriendshipToken,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordParams {
    qs_uid: QsUid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteUserRecordParams {
    qs_uid: QsUid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordParams {
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientRecordParams {
    qs_cid: QsCid,
    client_record_auth_key: SignaturePublicKey,
    queue_encryption_key: RatchetPublicKey,
    blocklist_entries: Vec<GroupId>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientRecordParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteClientRecordParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct PublishKeyPackagesParams {
    qs_cid: QsCid,
    add_packages: Vec<AddPackage>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchParams {
    qs_cid: QsCid,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DequeueMessagesParams {
    qs_cid: QsCid,
    sequence_number_start: u64,
    max_message_number: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct WsParams {
    qs_cid: QsCid,
}

// === Legacy ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct SendNonCommitParams {
    pub roster_key: RosterKdfKey,
    pub message: SerializedAssistedMessage, // Application message/Proposal
}

/// This is for the sender to create from SendNonCommitParams for serialization.
#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct TaggedSendNonCommitParams {
    payload: SendNonCommitParams,
    mac: MacTag,
}

impl TaggedStruct<SendNonCommitParams> for TaggedSendNonCommitParams {
    fn from_untagged_payload(payload: SendNonCommitParams, mac: MacTag) -> Self {
        Self { payload, mac }
    }
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct UpdateQueueConfigParams {
    roster_kdf_key: RosterKdfKey,
    group_id: GroupId,
    sender: LeafNodeIndex,
    new_queue_config: ClientQueueConfig,
}

impl UpdateQueueConfigParams {
    pub fn sender(&self) -> LeafNodeIndex {
        self.sender
    }

    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn new_queue_config(&self) -> &ClientQueueConfig {
        &self.new_queue_config
    }

    pub fn roster_kdf_key(&self) -> &RosterKdfKey {
        &self.roster_kdf_key
    }
}
