// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use mls_assist::openmls::prelude::SignaturePublicKey;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::{
    crypto::{
        ear::keys::FriendshipEarKey,
        signatures::keys::OwnerVerifyingKey,
        signatures::signable::{Signature, Verifiable, VerifiedStruct},
        QueueRatchet, RatchetPublicKey,
    },
    qs::{
        AddPackage, AddPackageIn, EncryptedPushToken, KeyPackageBatch, QsClientId,
        QsEncryptedAddPackage, QsUserId, VERIFIED,
    },
};

use super::{intra_backend::DsFanOutMessage, FriendshipToken, MlsInfraVersion, QueueMessage};

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
    pub owner_signature_key_option: Option<OwnerVerifyingKey>,
}

pub type QsInputMessage = DsFanOutMessage;

/// Error struct for deserialization of an [`UnverifiedGroupOperationParams`]
/// struct.
pub enum GroupOpsDeserializationError {
    DeserializationError,
    WrongRequestType,
}

// === User ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordParams {
    pub(crate) user_record_auth_key: OwnerVerifyingKey,
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_record_auth_key: OwnerVerifyingKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) add_packages: Vec<AddPackageIn>,
    pub(crate) friendship_ear_key: FriendshipEarKey,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
    pub(crate) initial_ratchet_key: QueueRatchet,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateUserRecordResponse {
    pub(crate) user_id: QsUserId,
    pub(crate) client_id: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UpdateUserRecordParams {
    pub(crate) sender: QsUserId,
    pub(crate) user_record_auth_key: OwnerVerifyingKey,
    pub(crate) friendship_token: FriendshipToken,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordParams {
    pub(crate) sender: QsUserId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct UserRecordResponse {
    pub(crate) friendship_token: FriendshipToken,
    pub(crate) client_records: Vec<ClientRecordResponse>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DeleteUserRecordParams {
    pub(crate) sender: QsUserId,
}

// === Client ===

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct CreateClientRecordParams {
    pub(crate) sender: QsUserId,
    pub(crate) client_record_auth_key: OwnerVerifyingKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) add_packages: Vec<AddPackageIn>,
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
    pub(crate) sender: QsClientId,
    pub(crate) client_record_auth_key: OwnerVerifyingKey,
    pub(crate) queue_encryption_key: RatchetPublicKey,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientRecordParams {
    pub(crate) sender: QsUserId,
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
    pub(crate) sender: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct PublishKeyPackagesParams {
    pub(crate) sender: QsClientId,
    pub(crate) add_packages: Vec<AddPackageIn>,
    pub(crate) friendship_ear_key: FriendshipEarKey,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageParams {
    pub(crate) sender: QsUserId,
    pub(crate) client_id: QsClientId,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct ClientKeyPackageResponse {
    pub(crate) encrypted_key_package: QsEncryptedAddPackage,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchParams {
    pub(crate) sender: FriendshipToken,
    pub(crate) friendship_ear_key: FriendshipEarKey,
}

#[derive(TlsSerialize, TlsSize, ToSchema)]
pub struct KeyPackageBatchResponse {
    pub(crate) add_packages: Vec<AddPackage>,
    pub(crate) key_package_batch: KeyPackageBatch<VERIFIED>,
}

#[derive(TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct DequeueMessagesParams {
    pub(crate) sender: QsClientId,
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

// === Auth & Framing ===

#[derive(TlsDeserialize, TlsSize)]
pub struct VerifiableClientToQsMessage {
    message: ClientToQsMessage,
    serialized_payload: Vec<u8>,
}

impl VerifiableClientToQsMessage {
    pub(crate) fn sender(&self) -> QsSender {
        self.message.sender()
    }

    // Verifies that the token matches the one in the message and returns the message.
    pub(crate) fn verify_with_token(self, token: FriendshipToken) -> Result<QsRequestParams, ()> {
        if matches!(self.sender(), QsSender::FriendshipToken(actual_token) if actual_token == token)
        {
            Ok(self.message.payload.body)
        } else {
            Err(())
        }
    }
}

impl Verifiable for VerifiableClientToQsMessage {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        Ok(self.serialized_payload.clone())
    }

    fn signature(&self) -> &Signature {
        &self.message.signature
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

#[derive(TlsDeserialize, TlsSize)]
pub(crate) struct ClientToQsMessage {
    payload: ClientToQsMessageTbs,
    // Signature over all of the above.
    signature: Signature,
}

impl ClientToQsMessage {
    pub(crate) fn sender(&self) -> QsSender {
        self.payload.sender()
    }
}

#[derive(TlsDeserialize, TlsSize)]
pub(crate) struct ClientToQsMessageTbs {
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
#[derive(TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(crate) enum QsRequestParams {
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
}

impl QsRequestParams {
    pub(crate) fn sender(&self) -> QsSender {
        match self {
            QsRequestParams::CreateUser(params) => {
                QsSender::OwnerVerifyingKey(params.user_record_auth_key.clone())
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
        }
    }
}

#[derive(TlsSize, TlsSerialize)]
#[repr(u8)]
pub enum QsProcessResponse {
    CreateUser(CreateUserRecordResponse),
    UpdateUser,
    DeleteUser,
    CreateClient(CreateClientRecordResponse),
    UpdateClient,
    DeleteClient,
    PublishKeyPackages,
    ClientKeyPackage(ClientKeyPackageResponse),
    KeyPackageBatch(KeyPackageBatchResponse),
    DequeueMessages(DequeueMessagesResponse),
}

pub enum QsSender {
    User(QsUserId),
    Client(QsClientId),
    FriendshipToken(FriendshipToken),
    OwnerVerifyingKey(OwnerVerifyingKey),
}
