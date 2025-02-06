// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed between clients and the backend.
//! TODO: We should eventually factor this module out, together with the crypto
//! module, to allow re-use by the client implementation.

use std::io;

use mls_assist::openmls::prelude::{KeyPackage, KeyPackageIn, SignaturePublicKey};
use thiserror::Error;
use tls_codec::{DeserializeBytes, Serialize, TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::keys::KeyPackageEarKey,
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
    keypackage_batch::{KeyPackageBatch, QsEncryptedKeyPackage, UNVERIFIED, VERIFIED},
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
    pub friendship_ear_key: KeyPackageEarKey,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientKeyPackageParams {
    pub sender: QsUserId,
    pub client_id: QsClientId,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientKeyPackageResponse {
    pub encrypted_key_package: QsEncryptedKeyPackage,
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct KeyPackageBatchParams {
    pub sender: FriendshipToken,
    pub friendship_ear_key: KeyPackageEarKey,
}

#[derive(Debug, TlsSerialize, TlsSize)]
pub struct KeyPackageBatchResponse {
    pub key_packages: Vec<KeyPackage>,
    pub key_package_batch: KeyPackageBatch<VERIFIED>,
}

#[derive(Debug, TlsSize, TlsDeserializeBytes)]
pub struct KeyPackageBatchResponseIn {
    pub key_packages: Vec<KeyPackageIn>,
    pub key_package_batch: KeyPackageBatch<UNVERIFIED>,
}

#[derive(Debug, TlsDeserializeBytes, TlsSerialize, TlsSize)]
#[cfg_attr(test, derive(Clone, PartialEq, Eq))]
pub struct VerifyingKeyResponse {
    pub verifying_key: QsVerifyingKey,
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
    pub fn sender(&self) -> Result<QsSender, VersionError> {
        self.message.sender()
    }

    // Verifies that the token matches the one in the message and returns the message.
    pub fn verify_with_token(
        self,
        token: FriendshipToken,
    ) -> Result<QsVersionedRequestParams, ClientToQsVerificationError> {
        if self.message.token_or_signature.as_slice() == token.token() {
            Ok(self.message.payload.body)
        } else {
            Err(ClientToQsVerificationError::InvalidToken)
        }
    }

    pub fn extract_without_verification(
        self,
    ) -> Result<QsVersionedRequestParams, ClientToQsVerificationError> {
        match self.message.payload.body {
            QsVersionedRequestParams::Alpha(
                QsRequestParams::VerifyingKey | QsRequestParams::EncryptionKey,
            ) => Ok(self.message.payload.body),
            QsVersionedRequestParams::Alpha(_) | QsVersionedRequestParams::Other(_) => {
                Err(ClientToQsVerificationError::ExtractionError)
            }
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

impl VerifiedStruct<VerifiableClientToQsMessage> for QsVersionedRequestParams {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableClientToQsMessage, _seal: Self::SealingType) -> Self {
        verifiable.message.payload.body
    }
}

/// QS Server API
#[derive(Debug, TlsDeserializeBytes, TlsSize)]
pub(crate) struct ClientToQsMessage {
    payload: ClientToQsMessageTbs,
    /// Signature over all of the above or friendship token or empty for messages
    /// without authentication
    token_or_signature: Signature,
}

impl ClientToQsMessage {
    pub(crate) fn sender(&self) -> Result<QsSender, VersionError> {
        self.payload.sender()
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct ClientToQsMessageTbs {
    // This essentially includes the wire format.
    body: QsVersionedRequestParams,
}

/// QS request parameters with attached API version
///
/// **WARNING**: Only add new variants with new API versions. Do not reuse the API version (variant
/// tag).
#[derive(Debug)]
#[repr(u16)]
pub enum QsVersionedRequestParams {
    /// Fallback for unknown versions
    Other(u16) = 0,
    Alpha(QsRequestParams) = 1,
}

impl tls_codec::Size for QsVersionedRequestParams {
    fn tls_serialized_len(&self) -> usize {
        match self {
            QsVersionedRequestParams::Alpha(params) => {
                1u16.tls_serialized_len() + params.tls_serialized_len()
            }
            QsVersionedRequestParams::Other(version) => version.tls_serialized_len(),
        }
    }
}

impl Serialize for QsVersionedRequestParams {
    fn tls_serialize<W: io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            QsVersionedRequestParams::Alpha(params) => {
                Ok(1u16.tls_serialize(writer)? + params.tls_serialize(writer)?)
            }
            QsVersionedRequestParams::Other(version) => version.tls_serialize(writer),
        }
    }
}

impl DeserializeBytes for QsVersionedRequestParams {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = u16::tls_deserialize_bytes(bytes)?;
        match version {
            1 => {
                let (params, bytes) = QsRequestParams::tls_deserialize_bytes(bytes)?;
                Ok((QsVersionedRequestParams::Alpha(params), bytes))
            }
            _ => Ok((QsVersionedRequestParams::Other(version), bytes)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Unsupported version: {version}, supported versions: {supported_versions:?}")]
pub struct VersionError {
    version: u16,
    supported_versions: Vec<MlsInfraVersion>,
}

impl VersionError {
    pub fn from_unsupported_version(version: u16) -> Self {
        Self {
            version,
            supported_versions: vec![MlsInfraVersion::Alpha],
        }
    }
}

impl ClientToQsMessageTbs {
    pub(crate) fn sender(&self) -> Result<QsSender, VersionError> {
        match &self.body {
            QsVersionedRequestParams::Alpha(params) => Ok(params.sender()),
            QsVersionedRequestParams::Other(version) => {
                Err(VersionError::from_unsupported_version(*version))
            }
        }
    }
}

/// This enum contains variants for each DS endpoint.
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
            QsRequestParams::UpdateUser(params) => QsSender::User(params.sender),
            QsRequestParams::DeleteUser(params) => QsSender::User(params.sender),
            QsRequestParams::CreateClient(params) => QsSender::User(params.sender),
            QsRequestParams::UpdateClient(params) => QsSender::Client(params.sender),
            QsRequestParams::DeleteClient(params) => QsSender::Client(params.sender),
            QsRequestParams::PublishKeyPackages(params) => QsSender::Client(params.sender),
            QsRequestParams::ClientKeyPackage(params) => QsSender::User(params.sender),
            QsRequestParams::KeyPackageBatch(params) => {
                QsSender::FriendshipToken(params.sender.clone())
            }
            QsRequestParams::DequeueMessages(params) => QsSender::Client(params.sender),
            QsRequestParams::EncryptionKey | QsRequestParams::VerifyingKey => QsSender::Anonymous,
        }
    }
}

#[derive(TlsSize)]
#[repr(u16)]
pub enum QsVersionedProcessResponse {
    Alpha(QsProcessResponse) = 1,
}

// Note: Manual implementation because `TlsSerialize` does not support custom variant tags.
impl tls_codec::Serialize for QsVersionedProcessResponse {
    fn tls_serialize<W: io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        match self {
            QsVersionedProcessResponse::Alpha(params) => {
                Ok(1u16.tls_serialize(writer)? + params.tls_serialize(writer)?)
            }
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

#[derive(TlsSize)]
#[repr(u16)]
pub enum QsVersionedProcessResponseIn {
    Other(u16) = 0,
    Alpha(QsProcessResponseIn) = 1,
}

impl tls_codec::DeserializeBytes for QsVersionedProcessResponseIn {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (version, bytes) = u16::tls_deserialize_bytes(bytes)?;
        match version {
            1 => {
                let (params, bytes) = QsProcessResponseIn::tls_deserialize_bytes(bytes)?;
                Ok((QsVersionedProcessResponseIn::Alpha(params), bytes))
            }
            _ => Ok((QsVersionedProcessResponseIn::Other(version), bytes)),
        }
    }
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use crate::{
        crypto::{
            ear::Ciphertext,
            signatures::{private_keys::VerifyingKey, signable::SignedStruct},
        },
        keypackage_batch::KeyPackageBatchTbs,
    };

    use super::*;

    #[test]
    fn create_user_api_stability() {
        let create_user_record_response = CreateUserRecordResponse {
            user_id: QsUserId::from(Uuid::from_u128(1)),
            client_id: QsClientId::from(Uuid::from_u128(2)),
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::CreateUser(
            create_user_record_response.clone(),
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::CreateUser(response)) => {
                assert_eq!(response, create_user_record_response);
            }
            _ => panic!("expected CreateUser variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }

    #[test]
    fn create_client_api_stability() {
        let create_client_record_response = CreateClientRecordResponse {
            client_id: QsClientId::from(Uuid::from_u128(1)),
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::CreateClient(
            create_client_record_response.clone(),
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::CreateClient(response)) => {
                assert_eq!(response, create_client_record_response);
            }
            _ => panic!("expected CreateClient variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }

    #[test]
    fn client_key_package_api_stability() {
        let client_key_package_response = ClientKeyPackageResponse {
            encrypted_key_package: QsEncryptedKeyPackage::from(Ciphertext::dummy()),
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::ClientKeyPackage(
            client_key_package_response,
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::ClientKeyPackage(_)) => {}
            _ => panic!("expected ClientKeyPackage variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }

    #[test]
    fn key_package_batch_api_stability() {
        let signed_at: DateTime<Utc> = "2023-01-01T00:00:00.000Z".parse().unwrap();
        let key_package_batch_reponse = KeyPackageBatchResponse {
            key_packages: vec![],
            key_package_batch: KeyPackageBatch::from_payload(
                KeyPackageBatchTbs::new("localhost".parse().unwrap(), vec![], signed_at.into()),
                Signature::from_bytes(b"signature".to_vec()),
            ),
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::KeyPackageBatch(
            key_package_batch_reponse,
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::KeyPackageBatch(_)) => {}
            _ => panic!("expected KeyPackageBatch variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }

    #[test]
    fn dequeue_messages_api_stability() {
        let dequeue_messages_response = DequeueMessagesResponse {
            messages: vec![
                QueueMessage {
                    sequence_number: 1,
                    ciphertext: Ciphertext::dummy(),
                },
                QueueMessage {
                    sequence_number: 2,
                    ciphertext: Ciphertext::dummy(),
                },
            ],
            remaining_messages_number: 42,
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::DequeueMessages(
            dequeue_messages_response.clone(),
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::DequeueMessages(response)) => {
                assert_eq!(response, dequeue_messages_response);
            }
            _ => panic!("expected DequeueMessages variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }

    #[test]
    fn verifying_key_api_stability() {
        let verifying_key_response = VerifyingKeyResponse {
            verifying_key: QsVerifyingKey::from(VerifyingKey::new_for_test(
                b"verifying_key".to_vec(),
            )),
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::VerifyingKey(
            verifying_key_response.clone(),
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::VerifyingKey(response)) => {
                assert_eq!(response, verifying_key_response);
            }
            _ => panic!("expected VerifyingKey variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }

    #[test]
    fn encryption_key_api_stability() {
        let encryption_key_response = EncryptionKeyResponse {
            encryption_key: ClientIdEncryptionKey::new_for_test(b"encryption_key".to_vec().into()),
        };
        let response = QsVersionedProcessResponse::Alpha(QsProcessResponse::EncryptionKey(
            encryption_key_response.clone(),
        ));
        let response_tls = response.tls_serialize_detached().unwrap();

        let response_in =
            QsVersionedProcessResponseIn::tls_deserialize_exact_bytes(&response_tls).unwrap();
        match response_in {
            QsVersionedProcessResponseIn::Alpha(QsProcessResponseIn::EncryptionKey(response)) => {
                assert_eq!(response, encryption_key_response);
            }
            _ => panic!("expected EncryptionKey variant"),
        }

        insta::assert_binary_snapshot!(".tls", response_tls);
    }
}
