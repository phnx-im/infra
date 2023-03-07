//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use hpke::HpkePublicKey;
use mls_assist::{KeyPackage, SignaturePublicKey};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        signatures::keys::QueueOwnerVerificationKey, signatures::signable::Signature,
        RatchetPublicKey,
    },
    qs::{client_record::QsClientRecord, ClientId, EncryptedPushToken, KeyPackageBatch, UserId},
};

use super::{intra_backend::DsFanOutMessage, AddPackage, FriendshipToken};

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
    pub client_id: ClientId,        // The target queue id.
    pub sequence_number_start: u64, // The sequence number of the first message we want to fetch.
    pub max_messages: u64, // The maximum number of messages we'd like to retrieve from the QS.
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsFetchMessagesResponse {
    pub messages: Vec<EnqueuedMessage>,
    pub remaining_messages: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsQueueUpdate {
    pub owner_public_key_option: Option<RatchetPublicKey>,
    pub owner_signature_key_option: Option<QueueOwnerVerificationKey>,
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
    pub(crate) user_record_auth_key: SignaturePublicKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_record_auth_key: SignaturePublicKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordResponse {
    pub(crate) user_id: UserId,
    pub(crate) client_id: ClientId,
    pub(crate) client_record: QsClientRecord,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateUserRecordParams {
    pub(crate) user_id: UserId,
    pub(crate) user_record_auth_key: SignaturePublicKey,
    pub(crate) friendship_token: FriendshipToken,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct UserRecordParams {
    pub(crate) user_id: UserId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct UserRecordResponse {
    pub(crate) user_record_auth_key: SignaturePublicKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_records: Vec<ClientRecordResponse>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct DeleteUserRecordParams {
    pub(crate) user_id: UserId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordParams {
    pub(crate) client_record_auth_key: SignaturePublicKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) key_packages: Vec<KeyPackage>,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordResponse {
    pub(crate) client_id: ClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateClientRecordParams {
    pub(crate) client_id: ClientId,
    pub(crate) client_record_auth_key: SignaturePublicKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientRecordParams {
    pub(crate) client_id: ClientId,
}

#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct ClientRecordResponse {
    pub(crate) client_record_auth_key: SignaturePublicKey,
    pub(crate) queue_encryption_key: HpkePublicKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteClientRecordParams {
    pub(crate) client_id: ClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct PublishKeyPackagesParams {
    pub(crate) client_id: ClientId,
    pub(crate) add_packages: Vec<AddPackage>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct ClientKeyPackageParams {
    pub(crate) client_id: ClientId,
}

pub(crate) struct KeyPackageBatchParams {
    pub(crate) client_id: ClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct KeyPackageBatchResponse {
    pub(crate) add_packages: Vec<AddPackage>,
    pub(crate) key_package_batch: KeyPackageBatch,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct DequeueMessagesParams {
    pub(crate) client_id: ClientId,
    pub(crate) sequence_number_start: u64,
    pub(crate) max_message_number: u64,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub(crate) struct WsParams {
    pub(crate) client_id: ClientId,
}
