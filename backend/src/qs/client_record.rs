// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    crypto::{
        ear::{keys::PushTokenEarKey, EarDecryptable},
        signatures::keys::QsClientVerifyingKey,
        DecryptionError, QueueRatchet, RatchetEncryptionKey, RatchetKeyUpdate,
    },
    ds::group_state::TimeStamp,
    messages::{intra_backend::DsFanOutPayload, QueueMessage},
    qs::WsNotification,
};

use super::{
    errors::EnqueueError, storage_provider_trait::QsStorageProvider, EncryptedPushToken, PushToken,
    QsClientId, QsUserId, WebsocketNotifier,
};

/// An enum defining the different kind of messages that are stored in an QS
/// queue.
/// TODO: This needs a codec that allows decoding to the proper type.
#[derive(Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize, Debug)]
#[repr(u8)]
pub(super) enum QsQueueMessage {
    #[tls_codec(discriminant = 1)]
    RatchetKeyUpdate(RatchetKeyUpdate),
    EnqueuedMessage(QueueMessage),
}

/// Info attached to a queue meant as a target for messages fanned out by a DS.
#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct QsClientRecord {
    pub user_id: QsUserId,
    pub(crate) encrypted_push_token: Option<EncryptedPushToken>,
    pub(crate) owner_public_key: RatchetEncryptionKey,
    pub(crate) owner_signature_key: QsClientVerifyingKey,
    pub(crate) current_ratchet_key: QueueRatchet,
    pub(crate) activity_time: TimeStamp,
}

impl QsClientRecord {
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
    pub(crate) async fn enqueue<S: QsStorageProvider, W: WebsocketNotifier>(
        &mut self,
        client_id: &QsClientId,
        storage_provider: &S,
        websocket_notifier: &W,
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

                // TODO: Store the new key.

                // TODO: Future work: PCS

                tracing::trace!("Enqueueing message in storage provider");
                storage_provider
                    .enqueue(client_id, queue_message)
                    .await
                    .map_err(EnqueueError::StorageProviderError::<S>)?;

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
                                .map_err(|e| match e {
                                    DecryptionError::DecryptionError => {
                                        EnqueueError::PushNotificationError
                                    }
                                })?;
                            // TODO: It's currently not clear where we store the alert level.
                            let alert_level = 0;
                            push_token.send_notification(alert_level);
                        }
                    }
                }
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
}
