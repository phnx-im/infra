// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::openmls::prelude::SignaturePublicKey;
use thiserror::Error;
use tls_codec::{Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::AddPackageEarKey,
        hpke::ClientIdEncryptionKey,
        kdf::keys::RatchetSecret,
        signatures::keys::QsClientVerifyingKey,
        signatures::{
            keys::{QsUserVerifyingKey, QsVerifyingKey},
            signable::{Signature, Verifiable, VerifiedStruct},
        },
        RatchetEncryptionKey,
    },
    identifiers::{QsClientId, QsUserId},
    keypackage_batch::{
        AddPackage, AddPackageIn, KeyPackageBatch, QsEncryptedAddPackage, UNVERIFIED, VERIFIED,
    },
};

use super::{push_token::EncryptedPushToken, FriendshipToken, MlsInfraVersion, QueueMessage};

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
    pub add_packages: Vec<AddPackageIn>,
    pub friendship_ear_key: AddPackageEarKey,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientKeyPackageParams {
    pub sender: QsUserId,
    pub client_id: QsClientId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientKeyPackageResponse {
    pub encrypted_key_package: QsEncryptedAddPackage,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct KeyPackageBatchParams {
    pub sender: FriendshipToken,
    pub friendship_ear_key: AddPackageEarKey,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct KeyPackageBatchResponse {
    pub add_packages: Vec<AddPackage>,
    pub key_package_batch: KeyPackageBatch<VERIFIED>,
}

#[derive(Debug, TlsSize, TlsDeserializeBytes)]
pub struct KeyPackageBatchResponseIn {
    pub add_packages: Vec<AddPackageIn>,
    pub key_package_batch: KeyPackageBatch<UNVERIFIED>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct VerifyingKeyResponse {
    pub verifying_key: QsVerifyingKey,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
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
pub struct DequeueMessagesResponse {
    pub messages: Vec<QueueMessage>,
    pub remaining_messages_number: u64,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub(crate) struct WsParams {
    pub(crate) client_id: QsClientId,
}

// === Auth & Framing ===

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct VerifiableClientToQsMessage {
    message: ClientToQsMessage,
}

#[derive(Debug, Error)]
pub enum ClientToQsVerificationError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Invalid message type for extration without verification")]
    ExtractionError,
}

impl VerifiableClientToQsMessage {
    pub fn sender(&self) -> QsSender {
        self.message.sender()
    }

    // Verifies that the token matches the one in the message and returns the message.
    pub fn verify_with_token(
        self,
        token: FriendshipToken,
    ) -> Result<QsRequestParams, ClientToQsVerificationError> {
        if self.message.token_or_signature.as_slice() == token.token() {
            Ok(self.message.payload.body)
        } else {
            Err(ClientToQsVerificationError::InvalidToken)
        }
    }

    pub fn extract_without_verification(
        self,
    ) -> Result<QsRequestParams, ClientToQsVerificationError> {
        if matches!(
            self.message.payload.body,
            QsRequestParams::VerifyingKey | QsRequestParams::EncryptionKey
        ) {
            Ok(self.message.payload.body)
        } else {
            Err(ClientToQsVerificationError::ExtractionError)
        }
    }
}

impl Verifiable for VerifiableClientToQsMessage {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.message.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.message.token_or_signature
    }

    fn label(&self) -> &str {
        "ClientToQsMessage"
    }
}

impl VerifiedStruct<VerifiableClientToQsMessage> for QsRequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToQsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub struct ClientToQsMessage {
    payload: ClientToQsMessageTbs,
    // Signature over all of the above or friendship token or empty for messages
    // without authentication
    token_or_signature: Signature,
}

impl ClientToQsMessage {
    pub(crate) fn sender(&self) -> QsSender {
        self.payload.sender()
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientToQsMessageTbs {
    version: MlsInfraVersion,
    // This essentially includes the wire format.
    body: QsRequestParams,
}

impl ClientToQsMessageTbs {
    pub(crate) fn sender(&self) -> QsSender {
        self.body.sender()
    }
}

/// This enum contains variatns for each DS endpoint.
#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsRequestParams {
    // User
    CreateUser(CreateUserRecordParams),
    UpdateUser(UpdateUserRecordParams),
    DeleteUser(DeleteUserRecordParams),
    // Client
    CreateClient(CreateClientRecordParams),
    UpdateClient(UpdateClientRecordParams),
    DeleteClient(DeleteClientRecordParams),
    // Key packages
    PublishKeyPackages(PublishKeyPackagesParams),
    ClientKeyPackage(ClientKeyPackageParams),
    KeyPackageBatch(KeyPackageBatchParams),
    // Messages
    DequeueMessages(DequeueMessagesParams),
    // Key material
    VerifyingKey,
    EncryptionKey,
}

impl QsRequestParams {
    pub(crate) fn sender(&self) -> QsSender {
        match self {
            QsRequestParams::CreateUser(params) => {
                QsSender::QsUserVerifyingKey(params.user_record_auth_key.clone())
            }
            QsRequestParams::UpdateUser(params) => QsSender::User(params.sender.clone()),
            QsRequestParams::DeleteUser(params) => QsSender::User(params.sender.clone()),
            QsRequestParams::CreateClient(params) => QsSender::User(params.sender.clone()),
            QsRequestParams::UpdateClient(params) => QsSender::Client(params.sender.clone()),
            QsRequestParams::DeleteClient(params) => QsSender::Client(params.sender.clone()),
            QsRequestParams::PublishKeyPackages(params) => QsSender::Client(params.sender.clone()),
            QsRequestParams::ClientKeyPackage(params) => QsSender::User(params.sender.clone()),
            QsRequestParams::KeyPackageBatch(params) => {
                QsSender::FriendshipToken(params.sender.clone())
            }
            QsRequestParams::DequeueMessages(params) => QsSender::Client(params.sender.clone()),
            QsRequestParams::EncryptionKey | QsRequestParams::VerifyingKey => QsSender::Anonymous,
        }
    }
}

#[derive(TlsSize, TlsSerialize)]
#[repr(u8)]
pub enum QsProcessResponse {
    Ok,
    CreateUser(CreateUserRecordResponse),
    CreateClient(CreateClientRecordResponse),
    ClientKeyPackage(ClientKeyPackageResponse),
    KeyPackageBatch(KeyPackageBatchResponse),
    DequeueMessages(DequeueMessagesResponse),
    VerifyingKey(VerifyingKeyResponse),
    EncryptionKey(EncryptionKeyResponse),
}

#[derive(Debug, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsProcessResponseIn {
    Ok,
    CreateUser(CreateUserRecordResponse),
    CreateClient(CreateClientRecordResponse),
    ClientKeyPackage(ClientKeyPackageResponse),
    KeyPackageBatch(KeyPackageBatchResponseIn),
    DequeueMessages(DequeueMessagesResponse),
    VerifyingKey(VerifyingKeyResponse),
    EncryptionKey(EncryptionKeyResponse),
}

#[derive(Debug)]
pub enum QsSender {
    User(QsUserId),
    Client(QsClientId),
    FriendshipToken(FriendshipToken),
    QsUserVerifyingKey(QsUserVerifyingKey),
    Anonymous,
}
