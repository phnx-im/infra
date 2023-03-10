// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains the APIs of the queuing service (QS). It only performs
//! a limited amount of rate-limiting, so it should only be deployed behind a
//! rate-limiting module.
//!
//! NOTE: This document and the API stubs in this module represent a work in
//! progress and will likely change in their details. However, barring the
//! discovery of a major flaw in the current design, the general design of the
//! QS should remain the same.
//!
//! # Overview
//!
//! The QS maintains the queues of clients of the homeserver and provides the
//! following functionalities:
//!
//! * queue creation by clients (although each client can only create a single
//!   queue)
//! * enqueuing of messages by delivery services (either local or remote) that
//!   are authorized to enqueue in a given queue
//! * dequeuing of messages by the owner of a given queue
//! * updating of queue information by the owner of a given queue
//! * notification of the queue owner upon message enqueuing, either via a
//!   Websocket or via a push token
//! * queue deletion either by the queue owner or by another authorized client
//!
//! # Encryption-at-rest
//!
//! To protect the metadata visible in MLS PublicMessages, the QS encrypts
//! messages in queues to the owning client. This is done using a simple
//! construction, where the owning client provides an HPKE public key to which
//! the QS can encrypt the symmetric key it uses to encrypt messages. This key
//! is occasionally updated by sampling a fresh key and using an HKDF to combine
//! it with the existing key. The fresh key is then encrypted to the HPKE key
//! and enqueued. Additionally, with each encryption, the key is ratcheted
//! forward using the same HKDF (but without fresh key material).
//!
//! # Queue creation
//!
//! Clients can create queues that are not associated with them and are
//! therefore pseudonymous.
//!
//! # Message enqueuing
//!
//! Delivery services that want to enqueue a message in a queue with a given
//! QueueID have to prove that they are authorized by the owner of the queue by
//! providing a signature over the enqueuing request.
//!
//! The QS then encrypts the messages (see above on how messages are encrypted
//! at rest), marks them with a sequence number and enqueues them.
//!
//! # Message dequeuing
//!
//! Clients that want to dequeue messages first have to authenticate themselves
//! as the owner of the given queue.
//!
//! They can then request messages with a given range of sequence numbers. When
//! receiving such a request, the QS deletes any messages with sequence numbers
//! smaller than the smalles requested one and responds with the requested
//! messages.

use crate::{
    crypto::{
        ear::{
            keys::{FriendshipEarKey, PushTokenEarKey},
            Ciphertext, EarEncryptable,
        },
        signatures::{
            keys::QsVerifyingKey,
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
            traits::SigningKey,
        },
        DecryptionPrivateKey, EncryptionPublicKey, HpkeCiphertext,
    },
    ds::group_state::TimeStamp,
    messages::intra_backend::DsFanOutMessage,
};

use async_trait::*;
use mls_assist::{KeyPackage, KeyPackageRef};
use serde::{Deserialize, Serialize};
use tls_codec::{
    Deserialize as TlsDeserializeTrait, Serialize as TlsSerializeTrait, TlsDeserialize,
    TlsSerialize, TlsSize,
};
use utoipa::ToSchema;

use self::errors::{QsEnqueueProviderError, SealError, UnsealError};

pub mod client_api;
pub mod client_record;
pub mod ds_api;
pub mod errors;
pub mod storage_provider_trait;
pub mod user_record;

#[derive(Serialize, Deserialize)]
struct PushToken {
    token: Vec<u8>,
}

impl PushToken {
    /// If the alert level is high enough, send a notification to the client.
    fn send_notification(&self, _alert_level: u8) {
        todo!()
    }
}
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct EncryptedPushToken {
    ctxt: Ciphertext,
}

impl AsRef<Ciphertext> for EncryptedPushToken {
    fn as_ref(&self) -> &Ciphertext {
        &self.ctxt
    }
}

impl From<Ciphertext> for EncryptedPushToken {
    fn from(ctxt: Ciphertext) -> Self {
        Self { ctxt }
    }
}

impl EarEncryptable<PushTokenEarKey, EncryptedPushToken> for PushToken {}

pub enum WebsocketNotifierError {
    WebsocketNotFound,
}

/// TODO: This should be unified with push notifications later
#[async_trait]
pub trait WebsocketNotifier {
    async fn notify(&self, client_id: &QsClientId) -> Result<(), WebsocketNotifierError>;
}

#[async_trait]
pub trait QsEnqueueProvider {
    async fn enqueue(&self, message: DsFanOutMessage) -> Result<(), QsEnqueueProviderError>;

    async fn verifying_key(&self, fqdn: &Fqdn) -> Result<QsVerifyingKey, QsEnqueueProviderError>;
}

#[derive(Debug)]
pub struct ClientIdDecryptionPrivateKey {
    private_key: DecryptionPrivateKey,
}

impl ClientIdDecryptionPrivateKey {
    pub(super) fn unseal_client_config(
        &self,
        sealed_client_reference: &SealedClientReference,
    ) -> Result<ClientConfig, UnsealError> {
        let bytes = self
            .private_key
            .decrypt(&[], &[], &sealed_client_reference.ciphertext)
            .map_err(|_| UnsealError::DecryptionError)?;
        ClientConfig::tls_deserialize(&mut bytes.as_slice()).map_err(|_| UnsealError::CodecError)
    }
}

#[derive(Debug, Clone)]
pub struct ClientIdEncryptionPublicKey {
    public_key: EncryptionPublicKey,
}

impl ClientIdEncryptionPublicKey {
    pub fn seal_client_config(
        &self,
        client_config: ClientConfig,
    ) -> Result<SealedClientReference, SealError> {
        let bytes = client_config
            .tls_serialize_detached()
            .map_err(|_| SealError::CodecError)?;
        let ciphertext = self
            .public_key
            .encrypt(&[], &[], &bytes)
            .map_err(|_| SealError::EncryptionError)?;
        Ok(SealedClientReference { ciphertext })
    }
}

/// This is the pseudonymous client id used on the QS.
#[derive(TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize, ToSchema, Clone)]
pub struct QsClientId {
    pub(crate) client_id: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsUserId {
    pub(crate) user_id: Vec<u8>,
}

/// Info describing the queue configuration for a member of a given group.
#[derive(TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize, ToSchema, Clone)]
pub struct ClientConfig {
    pub(crate) client_id: QsClientId,
    // Some clients might not use push tokens.
    pub(crate) push_token_ear_key: Option<PushTokenEarKey>,
}

impl ClientConfig {
    pub fn dummy_config() -> Self {
        Self {
            client_id: QsClientId {
                client_id: Vec::new(),
            },
            push_token_ear_key: None,
        }
    }
}

#[derive(Debug)]
struct QsSigningKey {
    signing_key: Vec<u8>,
}

impl AsRef<[u8]> for QsSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for QsSigningKey {}

// TODO: This could come from the storage provider
#[derive(Debug)]
pub struct Qs {
    signing_key: QsSigningKey,
    fqdn: Fqdn,
    client_id_private_key: ClientIdDecryptionPrivateKey,
    client_id_public_key: ClientIdEncryptionPublicKey,
}

#[derive(
    Clone,
    Serialize,
    Deserialize,
    ToSchema,
    TlsSerialize,
    TlsDeserialize,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
    Debug,
)]
pub struct Fqdn {}

#[derive(Clone, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsClientReference {
    client_homeserver_domain: Fqdn,
    sealed_reference: SealedClientReference,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct SealedClientReference {
    ciphertext: HpkeCiphertext,
}

// This is used to check keypackage batch freshness by the DS, so it's
// reasonable to assume the batch is relatively fresh.
pub const KEYPACKAGEBATCH_EXPIRATION_DAYS: i64 = 1;

/// Ciphertext that contains a KeyPackage and an intermediary client certficate.
/// TODO: do we want a key committing scheme here?
#[derive(Debug, TlsSerialize, TlsDeserialize, TlsSize, ToSchema)]
pub struct QsEncryptedAddPackage {
    ctxt: Ciphertext,
}

impl AsRef<Ciphertext> for QsEncryptedAddPackage {
    fn as_ref(&self) -> &Ciphertext {
        &self.ctxt
    }
}

impl From<Ciphertext> for QsEncryptedAddPackage {
    fn from(ctxt: Ciphertext) -> Self {
        Self { ctxt }
    }
}

impl EarEncryptable<FriendshipEarKey, QsEncryptedAddPackage> for AddPackage {}

#[derive(Debug, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct KeyPackageBatchTbs {
    homeserver_domain: Fqdn,
    key_package_refs: Vec<KeyPackageRef>,
    time_of_signature: TimeStamp,
}

impl KeyPackageBatchTbs {
    pub(crate) fn key_package_refs(&self) -> &[KeyPackageRef] {
        &self.key_package_refs
    }

    pub fn has_expired(&self, expiration_days: i64) -> bool {
        self.time_of_signature.has_expired(expiration_days)
    }
}

impl Signable for KeyPackageBatchTbs {
    type SignedOutput = KeyPackageBatch;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "KeyPackageBatch"
    }
}

impl SignedStruct<KeyPackageBatchTbs> for KeyPackageBatch {
    fn from_payload(payload: KeyPackageBatchTbs, signature: Signature) -> Self {
        KeyPackageBatch {
            key_package_refs: payload.key_package_refs,
            time_of_signature: payload.time_of_signature,
            signature,
        }
    }
}

#[derive(TlsDeserialize, TlsSize, ToSchema)]
pub struct VerifiableKeyPackageBatch {
    payload: KeyPackageBatchTbs,
    signature: Signature,
}

impl VerifiableKeyPackageBatch {
    pub(crate) fn homeserver_domain(&self) -> &Fqdn {
        &self.payload.homeserver_domain
    }
}

impl Verifiable for VerifiableKeyPackageBatch {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.payload.tls_serialize_detached()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn label(&self) -> &str {
        "KeyPackageBatchTBS"
    }
}

mod private_mod {
    #[derive(Default)]
    pub struct Seal;
}

impl VerifiedStruct<VerifiableKeyPackageBatch> for KeyPackageBatchTbs {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: VerifiableKeyPackageBatch, _seal: Self::SealingType) -> Self {
        verifiable.payload
    }
}

#[derive(Debug, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct KeyPackageBatch {
    key_package_refs: Vec<KeyPackageRef>,
    time_of_signature: TimeStamp,
    signature: Signature,
}

#[derive(Debug, ToSchema, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct AddPackage {
    key_package: KeyPackage,
    icc_ciphertext: Vec<u8>,
}
