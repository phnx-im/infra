//! This module contains the APIs of the queuing service (QS). It only performs
//! a limited amount of rate-limiting, so it should only be deployed behind a
//! rate-limiting module.
//!
//! NOTE: This document and the API stubs in this module represent a work in
//! progress and will likely change in their details. However, barring the
//! discovery of a major flaw in the current design, the general design of the
//! QS should remain the same.
//!
//! TODO: Do we want a flag that allows one layer of indirection, i.e. queue
//! aliases? This would allow users to hide metadata from the DS to a certain
//! degree and would help in the federated setting.
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
//! To protect the metadata visible in MLSPlaintext messages, the QS encrypts
//! messages in queues to the owning client. This is done using a simple
//! construction, where the owning client provides an HPKE public key to which
//! the QS can encrypt the symmetric key it uses to encrypt messages. This key
//! is occasionally updated by sampling a fresh key and using an HKDF to combine
//! it with the existing key. The fresh key is then encrypted to the HPKE key
//! and enqueued. Additionally, with each encryption, the key is ratcheted
//! forward using the same HKDF (but without fresh key material).
//!
//! TODO: Verify that this is what we meant to do here.
//!
//! # Queue creation
//!
//! TODO: I can't remember how we said we wanted to do queue creation. I think
//! we meant clients to be able to create only one queue, the deletion of which
//! would indicate the deletion of the client. Deletion was meant to be possible
//! via a specific key.
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
//!
//!
//!
use std::fmt::Display;

use crate::crypto::{
    ear::{keys::PushTokenEarKey, Ciphertext, EarEncryptable},
    DecryptionPrivateKey, EncryptionPublicKey,
};

use async_trait::*;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use utoipa::ToSchema;

pub mod as_api;
pub mod client_api;
pub mod ds_api;
pub mod errors;
pub mod queue_types;
pub mod storage_provider_trait;

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
    async fn notify(&self, queue_id: &QueueId) -> Result<(), WebsocketNotifierError>;
}

pub struct QueueIdDecryptionPrivateKey {
    private_key: DecryptionPrivateKey,
}

impl QueueIdDecryptionPrivateKey {
    pub(super) fn unseal_queue_config(
        &self,
        sealed_queue_config: &SealedQueueConfig,
    ) -> QueueConfig {
        todo!()
    }
}

pub struct QueueIdEncryptionPublicKey {
    public_key: EncryptionPublicKey,
}

impl QueueIdEncryptionPublicKey {
    // TODO: We might want this to be symmetric crypto instead.
    pub(super) fn seal_queue_config(&self, queue_config: QueueConfig) {
        todo!()
    }
}

/// An ID for a queue on an QS. This ID should be globally unique.
#[derive(
    TlsSerialize,
    TlsDeserialize,
    TlsSize,
    Serialize,
    Deserialize,
    ToSchema,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QueueId {
    pub id: Vec<u8>,
}

impl AsRef<[u8]> for QueueId {
    fn as_ref(&self) -> &[u8] {
        &self.id
    }
}

impl Display for QueueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.id)
    }
}

/// Info describing the queue configuration for a member of a given group.
#[derive(TlsSerialize, TlsDeserialize, TlsSize, Serialize, Deserialize, ToSchema, Clone)]
pub struct QueueConfig {
    // TODO: These values should not be public in the future
    pub queue_id: QueueId,
    // Some clients might not use push tokens.
    pub push_token_key_option: Option<PushTokenEarKey>,
}

impl QueueConfig {
    pub fn dummy_config() -> Self {
        Self {
            queue_id: QueueId { id: Vec::new() },
            push_token_key_option: None,
        }
    }
}

#[derive(Debug)]
pub struct Qs {}

#[derive(Clone, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct Fqdn {}

#[derive(Clone, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct ClientQueueConfig {
    client_homeserver_domain: Fqdn,
    sealed_config: SealedQueueConfig,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct SealedQueueConfig {}

#[derive(Debug, Serialize, Deserialize, ToSchema, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct KeyPackageBatch {}
