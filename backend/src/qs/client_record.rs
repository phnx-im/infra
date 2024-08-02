// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use phnxtypes::{
    crypto::{
        ear::{keys::PushTokenEarKey, EarDecryptable},
        ratchet::QueueRatchet,
        signatures::keys::QsClientVerifyingKey,
        RatchetEncryptionKey, RatchetKeyUpdate,
    },
    identifiers::{QsClientId, QsUserId},
    messages::{
        client_ds::QsQueueMessagePayload,
        push_token::{EncryptedPushToken, PushToken},
        EncryptedQsQueueMessage, QueueMessage,
    },
    time::TimeStamp,
};

use crate::{
    messages::intra_backend::DsFanOutPayload,
    qs::{PushNotificationError, WsNotification},
};

use super::{
    errors::EnqueueError, storage_provider_trait::QsStorageProvider, PushNotificationProvider,
    WebsocketNotifier,
};

/// An enum defining the different kind of messages that are stored in an QS
/// queue.
/// TODO: This needs a codec that allows decoding to the proper type.
#[derive(Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize, Debug)]
#[repr(u8)]
pub(super) enum QueueMessageType {
    #[tls_codec(discriminant = 1)]
    RatchetKeyUpdate(RatchetKeyUpdate),
    EnqueuedMessage(QueueMessage),
}

/// Info attached to a queue meant as a target for messages fanned out by a DS.
#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct QsClientRecord {
    pub user_id: QsUserId,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
    pub(crate) owner_public_key: RatchetEncryptionKey,
    pub(crate) owner_signature_key: QsClientVerifyingKey,
    pub(crate) current_ratchet_key: QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>,
    pub(crate) activity_time: TimeStamp,
}

impl QsClientRecord {
    /// Create a new client record.
    pub fn new(
        user_id: QsUserId,
        encrypted_push_token: Option<EncryptedPushToken>,
        owner_public_key: RatchetEncryptionKey,
        owner_signature_key: QsClientVerifyingKey,
        current_ratchet_key: QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>,
    ) -> Self {
        Self {
            user_id,
            encrypted_push_token,
            owner_public_key,
            owner_signature_key,
            current_ratchet_key,
            activity_time: TimeStamp::now(),
        }
    }

    /// This function is meant to be used to create a client record from value
    /// stored in a DB. To create a fresh QS client record, use `new`.
    pub fn from_db_values(
        user_id: QsUserId,
        encrypted_push_token: Option<EncryptedPushToken>,
        owner_public_key: RatchetEncryptionKey,
        owner_signature_key: QsClientVerifyingKey,
        current_ratchet_key: QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>,
        activity_time: TimeStamp,
    ) -> Self {
        Self {
            user_id,
            encrypted_push_token,
            owner_public_key,
            owner_signature_key,
            current_ratchet_key,
            activity_time,
        }
    }

    /// Update the client record.
    pub(crate) fn update(
        &mut self,
        client_record_auth_key: QsClientVerifyingKey,
        queue_encryption_key: RatchetEncryptionKey,
        encrypted_push_token: Option<EncryptedPushToken>,
    ) {
        self.owner_signature_key = client_record_auth_key;
        self.owner_public_key = queue_encryption_key;
        self.encrypted_push_token = encrypted_push_token;
        self.activity_time = TimeStamp::now();
    }

    /// Put a message into the queue.
    pub(crate) async fn enqueue<
        S: QsStorageProvider,
        W: WebsocketNotifier,
        P: PushNotificationProvider,
    >(
        &mut self,
        client_id: &QsClientId,
        storage_provider: &S,
        websocket_notifier: &W,
        push_token_provider: &P,
        msg: DsFanOutPayload,
        push_token_key_option: Option<PushTokenEarKey>,
    ) -> Result<(), EnqueueError<S>> {
        match msg {
            // Enqueue a queue message.
            // Serialize the message so that we can put it in the queue.
            // TODO: The message should be serialized differently, using a struct
            // with the sequence number
            DsFanOutPayload::QueueMessage(queue_message) => {
                // Encrypt the message under the current ratchet key.
                let queue_message = self
                    .current_ratchet_key
                    .encrypt(queue_message)
                    .map_err(|_| EnqueueError::LibraryError)?;

                // TODO: Future work: PCS

                tracing::trace!("Enqueueing message in storage provider");
                storage_provider
                    .enqueue(client_id, queue_message)
                    .await
                    .map_err(EnqueueError::StorageProviderEnqueueError::<S>)?;

                // Try to send a notification over the websocket, otherwise use push tokens if available
                if websocket_notifier
                    .notify(client_id, WsNotification::QueueUpdate)
                    .await
                    .is_err()
                {
                    // Send a push notification under the following conditions:
                    // - there is a push token associated with the queue
                    // - there is a push token decryption key
                    // - the decryption is successful
                    if let Some(ref encrypted_push_token) = self.encrypted_push_token {
                        if let Some(ref ear_key) = push_token_key_option {
                            let push_token = PushToken::decrypt(ear_key, encrypted_push_token)
                                .map_err(|_| EnqueueError::PushNotificationError)?;
                            // Send the push notification.
                            if let Err(e) = push_token_provider.push(push_token).await {
                                match e {
                                    // The push notification failed for some other reason.
                                    PushNotificationError::Other(error_description) => {
                                        tracing::error!(
                                            "Push notification failed unexpectedly: {}",
                                            error_description
                                        )
                                    }
                                    // The token is no longer valid and should be deleted.
                                    PushNotificationError::InvalidToken(error_description) => {
                                        tracing::info!(
                                            "Push notification failed because the token is invalid: {}",
                                            error_description
                                        );
                                        self.encrypted_push_token = None;
                                    }
                                    // There was a network error when trying to send the push notification.
                                    PushNotificationError::NetworkError(e) => tracing::info!(
                                        "Push notification failed because of a network error: {}",
                                        e
                                    ),
                                    PushNotificationError::UnsupportedType => tracing::warn!(
                                        "Push notification failed because the push token type is unsupported",
                                    ),
                                    PushNotificationError::JwtCreationError(e) => tracing::error!(
                                        "Push notification failed because the JWT token could not be created: {}",
                                        e
                                    ),
                                }
                            }
                        }
                    }
                }

                // We also update th client record in the storage provider,
                // since we need to store the new ratchet key and because we
                // might have deleted the push token.
                storage_provider
                    .store_client(client_id, self.clone())
                    .await
                    .map_err(EnqueueError::StorageProviderStoreClientError::<S>)?;
            }
            // Dispatch an event message.
            DsFanOutPayload::EventMessage(event_message) => {
                // We ignore the result, because dispatching events is best effort.œ
                let _ = websocket_notifier
                    .notify(client_id, WsNotification::Event(event_message))
                    .await;
            }
        }

        // Success!
        Ok(())
    }

    pub fn encrypted_push_token(&self) -> Option<&EncryptedPushToken> {
        self.encrypted_push_token.as_ref()
    }

    pub fn owner_public_key(&self) -> &RatchetEncryptionKey {
        &self.owner_public_key
    }

    pub fn owner_signature_key(&self) -> &QsClientVerifyingKey {
        &self.owner_signature_key
    }

    pub fn current_ratchet_key(
        &self,
    ) -> &QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload> {
        &self.current_ratchet_key
    }

    pub fn activity_time(&self) -> &TimeStamp {
        &self.activity_time
    }

    pub fn user_id(&self) -> &QsUserId {
        &self.user_id
    }
}
