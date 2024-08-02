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

use phnxtypes::{
    crypto::{
        errors::RandomnessError,
        signatures::{keys::QsVerifyingKey, traits::SigningKey},
    },
    identifiers::{Fqdn, QsClientId},
    messages::{client_ds::DsEventMessage, push_token::PushToken},
};

use async_trait::*;
use mls_assist::{
    openmls::prelude::{OpenMlsCrypto, OpenMlsProvider},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use serde::{Deserialize, Serialize};

use crate::messages::intra_backend::DsFanOutMessage;

pub mod client_api;
pub mod client_record;
pub mod ds_api;
pub mod errors;
pub mod network_provider_trait;
pub mod qs_api;
pub mod storage_provider_trait;
pub mod user_record;

pub enum WsNotification {
    Event(DsEventMessage),
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

#[derive(Debug)]
pub enum PushNotificationError {
    /// Just for logging.
    Other(String),
    /// The push token is invalid.
    InvalidToken(String),
    /// Network error.
    NetworkError(String),
    /// Unsupported type of push token.
    UnsupportedType,
    /// The JWT token for APNS could not be created.
    JwtCreationError(String),
}

#[async_trait]
pub trait PushNotificationProvider: std::fmt::Debug + Send + Sync + 'static {
    async fn push(&self, push_token: PushToken) -> Result<(), PushNotificationError>;
}

#[async_trait]
pub trait QsConnector: Sync + Send + std::fmt::Debug + 'static {
    type EnqueueError: std::fmt::Debug;
    type VerifyingKeyError;
    async fn dispatch(&self, message: DsFanOutMessage) -> Result<(), Self::EnqueueError>;
    async fn verifying_key(&self, domain: Fqdn) -> Result<QsVerifyingKey, Self::VerifyingKeyError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QsSigningKey {
    signing_key: Vec<u8>,
    verifiying_key: QsVerifyingKey,
}

impl QsSigningKey {
    pub fn generate() -> Result<Self, RandomnessError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        let (signing_key, verifying_key) = rust_crypto
            .crypto()
            .signature_key_gen(mls_assist::openmls::prelude::SignatureScheme::ED25519)
            .map_err(|_| RandomnessError::InsufficientRandomness)?;
        let key = Self {
            signing_key,
            verifiying_key: QsVerifyingKey::new(verifying_key),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QsConfig {
    pub domain: Fqdn,
}

#[derive(Debug)]
pub struct Qs {}
