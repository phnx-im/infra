//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::{KeyPackage, SignaturePublicKey};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::keys::FriendshipEarKey, signatures::keys::QueueOwnerVerifyingKey,
        signatures::signable::Signature, QueueRatchet, RatchetPublicKey,
    },
    qs::{EncryptedPushToken, KeyPackageBatch, QsClientId, QsEncryptedKeyPackage, UserId},
};

use super::{
    client_ds::EncryptedDsMessage, intra_backend::DsFanOutMessage, FriendshipToken, MlsInfraVersion,
};

mod private_mod {
    #[derive(Default)]
    pub(crate) struct Seal;
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsFetchMessagesParams {
    pub payload: QsFetchMessageParamsTBS,
    pub signature: Signature, // A signature over the whole request using the queue owner's private key.
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsFetchMessageParamsTBS {
    pub client_id: QsClientId,      // The target queue id.
    pub sequence_number_start: u64, // The sequence number of the first message we want to fetch.
    pub max_messages: u64, // The maximum number of messages we'd like to retrieve from the QS.
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsFetchMessagesResponse {
    pub messages: Vec<QueueMessage>,
    pub remaining_messages: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsQueueUpdate {
    pub owner_public_key_option: Option<RatchetPublicKey>,
    pub owner_signature_key_option: Option<QueueOwnerVerifyingKey>,
}

pub type QsInputMessage = DsFanOutMessage;

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QueueMessage {
    pub(crate) sequence_number: u64,
    pub(crate) ciphertext: EncryptedDsMessage,
}

/// Error struct for deserialization of an [`UnverifiedGroupOperationParams`]
/// struct.
pub enum GroupOpsDeserializationError {
    DeserializationError,
    WrongRequestType,
}

// === User ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordParams {
    pub(crate) user_record_auth_key: SignaturePublicKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_record_auth_key: QueueOwnerVerifyingKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) key_packages: Vec<KeyPackage>,
    pub(crate) friendship_ear_key: FriendshipEarKey,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
    pub(crate) initial_ratchet_key: QueueRatchet,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordResponse {
    pub(crate) user_id: UserId,
    pub(crate) client_id: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateUserRecordParams {
    pub(crate) user_id: UserId,
    pub(crate) user_record_auth_key: SignaturePublicKey,
    pub(crate) friendship_token: FriendshipToken,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordParams {
    pub(crate) user_id: UserId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordResponse {
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_records: Vec<ClientRecordResponse>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteUserRecordParams {
    pub(crate) user_id: UserId,
}

// === Client ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordParams {
    pub(crate) sender: UserId,
    pub(crate) client_record_auth_key: QueueOwnerVerifyingKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) key_packages: Vec<KeyPackage>,
    pub(crate) friendship_ear_key: FriendshipEarKey,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
    pub(crate) initial_ratchet_key: QueueRatchet, // TODO: This can be dropped once we support PCS
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordResponse {
    pub(crate) client_id: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientRecordParams {
    pub(crate) client_id: QsClientId,
    pub(crate) client_record_auth_key: QueueOwnerVerifyingKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientRecordParams {
    pub(crate) client_id: QsClientId,
}

//pub type ClientRecordResponse = QsClientRecord;

#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct ClientRecordResponse {
    pub(crate) client_record_auth_key: SignaturePublicKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteClientRecordParams {
    pub(crate) client_id: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct PublishKeyPackagesParams {
    pub(crate) client_id: QsClientId,
    pub(crate) key_packages: Vec<KeyPackage>,
    pub(crate) friendship_ear_key: FriendshipEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageParams {
    pub(crate) client_id: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageResponse {
    pub(crate) encrypted_key_package: QsEncryptedKeyPackage,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchParams {
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) friendship_ear_key: FriendshipEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchResponse {
    pub(crate) key_packages: Vec<KeyPackage>,
    pub(crate) key_package_batch: KeyPackageBatch,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DequeueMessagesParams {
    pub(crate) client_id: QsClientId,
    pub(crate) sequence_number_start: u64,
    pub(crate) max_message_number: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DequeueMessagesResponse {
    pub(crate) messages: Vec<QueueMessage>,
    pub(crate) remaining_messages_number: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct WsParams {
    pub(crate) client_id: QsClientId,
}

// === Client messages ===

#[derive(TlsDeserialize, TlsSize)]
// TODO: This needs custom TLS Codec functions.
pub struct VerifiableClientToQsMessage {
    message: ClientToQsMessage,
    serialized_payload: Vec<u8>,
}

#[derive(TlsDeserialize, TlsSize)]
pub(crate) struct ClientToQsMessage {
    payload: ClientToQsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
}

impl ClientToQsMessage {}

#[derive(TlsDeserialize, TlsSize)]
pub(crate) struct ClientToQsMessageTbs {
    version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: QsRequestParams,
}

impl ClientToQsMessageTbs {}

/// This enum contains variatns for each DS endpoint.
#[derive(TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(crate) enum QsRequestParams {
    // User
    CreateUser(CreateUserRecordParams),
    UpdateUser(UpdateUserRecordParams),
}

impl QsRequestParams {}
