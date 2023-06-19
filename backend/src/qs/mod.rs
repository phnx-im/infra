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
            keys::{AddPackageEarKey, PushTokenEarKey},
            Ciphertext, EarDecryptable, EarEncryptable,
        },
        hpke::{HpkeDecryptable, HpkeDecryptionKey, HpkeEncryptable, HpkeEncryptionKey},
        signatures::{
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
            traits::{SigningKey, VerifyingKey},
        },
        DecryptionPrivateKey, EncryptionPublicKey, RandomnessError,
    },
    ds::group_state::{EncryptedClientCredential, TimeStamp},
    messages::{client_ds::EventMessage, intra_backend::DsFanOutMessage},
};

use async_trait::*;
use mls_assist::{
    openmls::prelude::{
        KeyPackage, KeyPackageIn, KeyPackageRef, KeyPackageVerifyError, OpenMlsCrypto,
        OpenMlsCryptoProvider, OpenMlsRand, ProtocolVersion,
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::types::HpkeCiphertext,
};
use serde::{Deserialize, Serialize};
use tls_codec::{
    DeserializeBytes as TlsDeserializeBytesTrait, Serialize as TlsSerializeTrait,
    TlsDeserializeBytes, TlsSerialize, TlsSize,
};
use utoipa::ToSchema;

use self::errors::SealError;

pub mod client_api;
pub mod client_record;
pub mod ds_api;
pub mod errors;
pub mod storage_provider_trait;
pub mod user_record;

#[derive(Serialize, Deserialize)]
pub struct PushToken {
    token: Vec<u8>,
}

impl PushToken {
    // TODO: This is a dummy implementation for now
    pub fn dummy() -> Self {
        Self { token: vec![0; 32] }
    }

    /// If the alert level is high enough, send a notification to the client.
    fn send_notification(&self, _alert_level: u8) {
        todo!()
    }
}
#[derive(
    Serialize, Deserialize, ToSchema, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
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
impl EarDecryptable<PushTokenEarKey, EncryptedPushToken> for PushToken {}

pub enum WsNotification {
    Event(EventMessage),
    QueueUpdate,
}

#[derive(Debug)]
pub enum WebsocketNotifierError {
    WebsocketNotFound,
}

/// TODO: This should be unified with push notifications later
#[async_trait]
pub trait WebsocketNotifier {
    async fn notify(
        &self,
        client_id: &QsClientId,
        ws_notification: WsNotification,
    ) -> Result<(), WebsocketNotifierError>;
}

#[async_trait]
pub trait QsConnector: Sync + Send + std::fmt::Debug + 'static {
    type EnqueueError;
    type VerifyingKeyError;
    async fn dispatch(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError>;
    async fn verifying_key(&self, fqdn: &Fqdn) -> Result<QsVerifyingKey, Self::VerifyingKeyError>;
}

#[derive(Debug, Clone)]
pub struct ClientIdDecryptionKey {
    private_key: DecryptionPrivateKey,
    encryption_key: ClientIdEncryptionKey,
}

impl AsRef<DecryptionPrivateKey> for ClientIdDecryptionKey {
    fn as_ref(&self) -> &DecryptionPrivateKey {
        &self.private_key
    }
}

impl HpkeDecryptionKey for ClientIdDecryptionKey {}

impl ClientIdDecryptionKey {
    //pub(super) fn unseal_client_config(
    //    &self,
    //    sealed_client_reference: &SealedClientReference,
    //) -> Result<ClientConfig, UnsealError> {
    //    let bytes = self
    //        .private_key
    //        .decrypt(&[], &[], &sealed_client_reference.ciphertext)
    //        .map_err(|_| UnsealError::DecryptionError)?;
    //    ClientConfig::tls_deserialize_exact(&bytes).map_err(|_| UnsealError::CodecError)
    //}

    pub fn generate() -> Result<Self, RandomnessError> {
        let private_key = DecryptionPrivateKey::generate()?;
        let encryption_key = ClientIdEncryptionKey {
            public_key: private_key.public_key().clone(),
        };
        Ok(Self {
            private_key,
            encryption_key,
        })
    }

    pub fn encryption_key(&self) -> &ClientIdEncryptionKey {
        &self.encryption_key
    }
}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct ClientIdEncryptionKey {
    public_key: EncryptionPublicKey,
}

impl AsRef<EncryptionPublicKey> for ClientIdEncryptionKey {
    fn as_ref(&self) -> &EncryptionPublicKey {
        &self.public_key
    }
}

impl HpkeEncryptionKey for ClientIdEncryptionKey {}

impl ClientIdEncryptionKey {
    pub fn seal_client_config(
        &self,
        client_config: ClientConfig,
    ) -> Result<SealedClientReference, SealError> {
        let bytes = client_config
            .tls_serialize_detached()
            .map_err(|_| SealError::CodecError)?;
        let ciphertext = self.public_key.encrypt(&[], &[], &bytes);
        Ok(SealedClientReference { ciphertext })
    }
}

/// This is the pseudonymous client id used on the QS.
#[derive(
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    Serialize,
    Deserialize,
    ToSchema,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QsClientId {
    pub(crate) client_id: Vec<u8>,
}

impl QsClientId {
    pub fn from_bytes(client_id: Vec<u8>) -> Self {
        Self { client_id }
    }

    pub fn random() -> Self {
        let client_id = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        Self { client_id }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.client_id
    }
}

#[derive(
    Clone,
    Debug,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QsUserId {
    pub(crate) user_id: Vec<u8>,
}

impl QsUserId {
    pub fn random() -> Self {
        let user_id = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        Self { user_id }
    }
}

/// Info describing the queue configuration for a member of a given group.
#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize, ToSchema, Clone)]
pub struct ClientConfig {
    pub client_id: QsClientId,
    // Some clients might not use push tokens.
    pub push_token_ear_key: Option<PushTokenEarKey>,
}

impl HpkeEncryptable<ClientIdEncryptionKey, SealedClientReference> for ClientConfig {}
impl HpkeDecryptable<ClientIdDecryptionKey, SealedClientReference> for ClientConfig {}

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

#[derive(Debug, Clone)]
pub struct QsSigningKey {
    signing_key: Vec<u8>,
    verifiying_key: QsVerifyingKey,
}

impl QsSigningKey {
    pub fn generate() -> Result<Self, RandomnessError> {
        let backend = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = backend
            .crypto()
            .signature_key_gen(mls_assist::openmls::prelude::SignatureScheme::ED25519)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        let key = Self {
            signing_key,
            verifiying_key: QsVerifyingKey { verifying_key },
        };
        Ok(key)
    }

    pub fn verifying_key(&self) -> &QsVerifyingKey {
        &self.verifiying_key
    }
}

impl AsRef<[u8]> for QsSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key
    }
}

impl SigningKey for QsSigningKey {}

#[derive(Debug, Clone, TlsDeserializeBytes, TlsSerialize, TlsSize)]
pub struct QsVerifyingKey {
    verifying_key: Vec<u8>,
}

impl AsRef<[u8]> for QsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.verifying_key
    }
}

impl VerifyingKey for QsVerifyingKey {}

#[derive(Debug, Clone)]
pub struct QsConfig {
    pub fqdn: Fqdn,
}

#[derive(Debug)]
pub struct Qs {}

#[derive(
    Clone,
    Serialize,
    Deserialize,
    ToSchema,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
    Debug,
)]
pub struct Fqdn {}

#[derive(
    Clone,
    Serialize,
    Deserialize,
    ToSchema,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QsClientReference {
    pub client_homeserver_domain: Fqdn,
    pub sealed_reference: SealedClientReference,
}

#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    Clone,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    Hash,
)]
pub struct SealedClientReference {
    ciphertext: HpkeCiphertext,
}

impl From<HpkeCiphertext> for SealedClientReference {
    fn from(value: HpkeCiphertext) -> Self {
        Self { ciphertext: value }
    }
}

impl AsRef<HpkeCiphertext> for SealedClientReference {
    fn as_ref(&self) -> &HpkeCiphertext {
        &self.ciphertext
    }
}

// This is used to check keypackage batch freshness by the DS, so it's
// reasonable to assume the batch is relatively fresh.
pub const KEYPACKAGEBATCH_EXPIRATION_DAYS: i64 = 1;

/// Ciphertext that contains a KeyPackage and an intermediary client certficate.
/// TODO: do we want a key committing scheme here?
#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema, Clone)]
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

impl EarEncryptable<AddPackageEarKey, QsEncryptedAddPackage> for AddPackage {}

#[derive(
    Clone, Debug, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct KeyPackageBatchTbs {
    homeserver_domain: Fqdn,
    key_package_refs: Vec<KeyPackageRef>,
    time_of_signature: TimeStamp,
}

impl Signable for KeyPackageBatchTbs {
    type SignedOutput = KeyPackageBatch<VERIFIED>;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        "KeyPackageBatch"
    }
}

impl SignedStruct<KeyPackageBatchTbs> for KeyPackageBatch<VERIFIED> {
    fn from_payload(payload: KeyPackageBatchTbs, signature: Signature) -> Self {
        KeyPackageBatch { payload, signature }
    }
}

pub const VERIFIED: bool = true;
pub const UNVERIFIED: bool = false;

impl KeyPackageBatch<UNVERIFIED> {
    pub(crate) fn homeserver_domain(&self) -> &Fqdn {
        &self.payload.homeserver_domain
    }
}

impl Verifiable for KeyPackageBatch<UNVERIFIED> {
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

impl VerifiedStruct<KeyPackageBatch<UNVERIFIED>> for KeyPackageBatch<VERIFIED> {
    type SealingType = private_mod::Seal;

    fn from_verifiable(verifiable: KeyPackageBatch<UNVERIFIED>, _seal: Self::SealingType) -> Self {
        KeyPackageBatch::<VERIFIED> {
            payload: verifiable.payload,
            signature: verifiable.signature,
        }
    }
}

#[derive(Clone, Debug, ToSchema, TlsSerialize, TlsSize)]
pub struct KeyPackageBatch<const IS_VERIFIED: bool> {
    payload: KeyPackageBatchTbs,
    signature: Signature,
}

impl KeyPackageBatch<VERIFIED> {
    pub(crate) fn key_package_refs(&self) -> &[KeyPackageRef] {
        &self.payload.key_package_refs
    }

    pub fn has_expired(&self, expiration_days: i64) -> bool {
        self.payload.time_of_signature.has_expired(expiration_days)
    }
}

impl TlsDeserializeBytesTrait for KeyPackageBatch<UNVERIFIED> {
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error>
    where
        Self: Sized,
    {
        let (payload, bytes) = KeyPackageBatchTbs::tls_deserialize(bytes)?;
        let (signature, bytes) = Signature::tls_deserialize(bytes)?;
        Ok((Self { payload, signature }, bytes))
    }
}

#[derive(Debug, ToSchema, Serialize, Deserialize, TlsSerialize, TlsSize)]
pub struct AddPackage {
    key_package: KeyPackage,
    encrypted_client_credential: EncryptedClientCredential,
}

impl AddPackage {
    pub fn new(
        key_package: KeyPackage,
        encrypted_client_credential: EncryptedClientCredential,
    ) -> Self {
        Self {
            key_package,
            encrypted_client_credential,
        }
    }

    pub fn key_package(&self) -> &KeyPackage {
        &self.key_package
    }
}

#[derive(Debug, ToSchema, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct AddPackageIn {
    key_package: KeyPackageIn,
    encrypted_client_credential: EncryptedClientCredential,
}

impl AddPackageIn {
    pub fn validate(
        self,
        crypto: &impl OpenMlsCrypto,
        protocol_version: ProtocolVersion,
    ) -> Result<AddPackage, KeyPackageVerifyError> {
        let key_package = self.key_package.validate(crypto, protocol_version)?;
        Ok(AddPackage {
            key_package,
            encrypted_client_credential: self.encrypted_client_credential,
        })
    }
}

impl EarDecryptable<AddPackageEarKey, QsEncryptedAddPackage> for AddPackageIn {}
