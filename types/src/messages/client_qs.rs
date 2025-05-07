// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::openmls::prelude::{KeyPackage, KeyPackageIn, SignaturePublicKey};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        RatchetEncryptionKey,
        hpke::ClientIdEncryptionKey,
        kdf::keys::RatchetSecret,
        signatures::{
            keys::{QsClientVerifyingKey, QsUserVerifyingKey},
            signable::Signature,
        },
    },
    identifiers::{QsClientId, QsUserId},
};

use super::{FriendshipToken, QueueMessage, push_token::EncryptedPushToken};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct QsOpenWsParams {
    pub queue_id: QsClientId,
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct QsFetchMessagesParams {
    pub payload: QsFetchMessageParamsTBS,
    pub signature: Signature, // A signature over the whole request using the queue owner's private key.
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct QsFetchMessageParamsTBS {
    pub client_id: QsClientId,      // The target queue id.
    pub sequence_number_start: u64, // The sequence number of the first message we want to fetch.
    pub max_messages: u64, // The maximum number of messages we'd like to retrieve from the QS.
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct QsFetchMessagesResponse {
    pub messages: Vec<QueueMessage>,
    pub remaining_messages: u64,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct QsQueueUpdate {
    pub owner_public_key_option: Option<RatchetEncryptionKey>,
    pub owner_signature_key_option: Option<QsClientVerifyingKey>,
}

// === User ===

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct CreateUserRecordParams {
    pub user_record_auth_key: QsUserVerifyingKey,
    pub friendship_token: FriendshipToken,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[cfg_attr(test, derive(Clone, PartialEq, Eq))]
pub struct CreateUserRecordResponse {
    pub user_id: QsUserId,
    pub client_id: QsClientId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UpdateUserRecordParams {
    pub sender: QsUserId,
    pub user_record_auth_key: QsUserVerifyingKey,
    pub friendship_token: FriendshipToken,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UserRecordParams {
    pub(crate) sender: QsUserId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UserRecordResponse {
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_records: Vec<ClientRecordResponse>,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct DeleteUserRecordParams {
    pub sender: QsUserId,
}

// === Client ===

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct CreateClientRecordParams {
    pub sender: QsUserId,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
    pub initial_ratchet_secret: RatchetSecret, // TODO: This can be dropped once we support PCS
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[cfg_attr(test, derive(Clone, PartialEq, Eq))]
pub struct CreateClientRecordResponse {
    pub client_id: QsClientId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct UpdateClientRecordParams {
    pub sender: QsClientId,
    pub client_record_auth_key: QsClientVerifyingKey,
    pub queue_encryption_key: RatchetEncryptionKey,
    pub encrypted_push_token: Option<EncryptedPushToken>,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientRecordParams {
    pub(crate) sender: QsUserId,
    pub(crate) client_id: QsClientId,
}

//pub type ClientRecordResponse = QsClientRecord;

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub(crate) struct ClientRecordResponse {
    pub(crate) client_record_auth_key: SignaturePublicKey,
    pub(crate) queue_encryption_key: RatchetEncryptionKey,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct DeleteClientRecordParams {
    pub sender: QsClientId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct PublishKeyPackagesParams {
    pub sender: QsClientId,
    pub key_packages: Vec<KeyPackageIn>,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct KeyPackageParams {
    pub sender: FriendshipToken,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct KeyPackageResponse {
    pub key_package: KeyPackage,
}

#[derive(Debug, TlsSize, TlsDeserializeBytes)]
pub struct KeyPackageResponseIn {
    pub key_package: KeyPackageIn,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
#[cfg_attr(test, derive(Clone, PartialEq, Eq))]
pub struct EncryptionKeyResponse {
    pub encryption_key: ClientIdEncryptionKey,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct DequeueMessagesParams {
    pub sender: QsClientId,
    pub sequence_number_start: u64,
    pub max_message_number: u64,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[cfg_attr(test, derive(Clone, PartialEq, Eq))]
pub struct DequeueMessagesResponse {
    pub messages: Vec<QueueMessage>,
    pub remaining_messages_number: u64,
}

#[derive(Debug)]
pub enum QsSender {
    User(QsUserId),
    Client(QsClientId),
    FriendshipToken(FriendshipToken),
    QsUserVerifyingKey(QsUserVerifyingKey),
    Anonymous,
}
