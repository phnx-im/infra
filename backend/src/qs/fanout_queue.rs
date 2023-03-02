use mls_assist::messages::SerializedAssistedMessage;
use serde::{Deserialize, Serialize};
use tls_codec::{Serialize as TlsSerializeTrait, TlsDeserialize, TlsSerialize, TlsSize};
use tracing::instrument;

use crate::{
    crypto::{
        ear::{keys::PushTokenEarKey, DecryptionError, EarEncryptable},
        mac::keys::{EnqueueAuthKeyCtxt, QueueDeletionAuthKeyCtxt},
        signatures::signable::Signature,
        signatures::{keys::QueueOwnerVerificationKey, traits::SignatureVerificationError},
        RatchetKey, RatchetKeyUpdate, RatchetPublicKey,
    },
    messages::client_qs::{EnqueuedMessage, QsFanOutQueueUpdate},
};

use super::{
    errors::EnqueueFanOutError, storage_provider_trait::QsStorageProvider, EncryptedPushToken,
    PushToken, QueueId, WebsocketNotifier,
};

/// An enum defining the different kind of messages that are stored in an QS
/// queue.
/// TODO: This needs a codec that allows decoding to the proper type.
#[derive(Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
#[repr(u8)]
pub(super) enum QsQueueMessage {
    #[tls_codec(discriminant = 1)]
    RatchetKeyUpdate(RatchetKeyUpdate),
    EnqueuedMessage(EnqueuedMessage),
}

/// Info attached to a queue meant as a target for messages fanned out by an
/// DS.
/// TODO: Replace individual EAR keys with a joint EAR key for both auth key and
/// push token.
/// TODO: If we want to be able to roll EAR keys, we need to maintain an old
/// ciphertext, so that we have time to propagate the new key material.
#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct FanOutQueueInfo {
    encrypted_push_token_option: Option<EncryptedPushToken>,
    // Encrypted key that authenticates entities that want to deposit messages
    // in the queue.
    encrypted_enqueue_auth_key: EnqueueAuthKeyCtxt,
    owner_public_key: RatchetPublicKey,
    owner_signature_key: QueueOwnerVerificationKey,
    current_ratchet_key: RatchetKey,
    // Encrypted key that authenticates entities that want to delete the queue.
    encrypted_delete_auth_key: QueueDeletionAuthKeyCtxt,
}

impl FanOutQueueInfo {
    /// Verify the request against the signature key of the queue owner. Returns
    /// an error if the authentication fails.
    #[instrument(level = "trace", skip_all, err)]
    pub(crate) fn verify_against_owner_key(
        &self,
        _signature: &Signature,
    ) -> Result<(), SignatureVerificationError> {
        // TODO: This should verify a QsAuthToken instead of a signature over a
        // request hash.
        //self.basic_queue_info
        //    .owner_signature_key
        //    .verify(request_hash, signature)
        //    .map_err(|_| VerificationError::VerificationFailure)
        todo!()
    }

    pub(crate) fn encrypted_delete_auth_key(&self) -> &QueueDeletionAuthKeyCtxt {
        &self.encrypted_delete_auth_key
    }
}

impl FanOutQueueInfo {
    /// Put a message into the queue.
    pub(crate) async fn enqueue<S: QsStorageProvider, W: WebsocketNotifier>(
        &mut self,
        queue_id: &QueueId,
        storage_provider: &S,
        websocket_notifier: &W,
        msg: SerializedAssistedMessage,
        push_token_key_option: Option<PushTokenEarKey>,
    ) -> Result<(), EnqueueFanOutError<S>> {
        // Serialize the message so that we can put it in the queue.
        let message_bytes =
        // serialization shouldn't fail
        msg.tls_serialize_detached().map_err(|_| EnqueueFanOutError::LibraryError)?;

        // TODO: The message should be serialized differently, using a struct
        // with the sequence number

        // Encrypt the message under the current ratchet key.
        let encrypted_message = self.current_ratchet_key.encrypt(&message_bytes);

        // Ratchet the current ratchet key forward.
        let _ratchet_key_update = self.current_ratchet_key.ratchet_forward();

        tracing::trace!("Enqueueing message in storage provider");
        storage_provider
            .enqueue(queue_id, encrypted_message)
            .await
            .map_err(EnqueueFanOutError::StorageProviderError::<S>)?;

        // Try to send a notification over the websocket, otherwise use push tokens if available
        if websocket_notifier.notify(queue_id).await.is_err() {
            // Send a push notification under the following conditions:
            // - there is a push token associated with the queue
            // - there is a push token decryption key
            // - the decryption is successful
            if let Some(ref encrypted_push_token) = self.encrypted_push_token_option {
                if let Some(ref ear_key) = push_token_key_option {
                    let push_token =
                        PushToken::decrypt(ear_key, encrypted_push_token).map_err(|e| match e {
                            DecryptionError::DecryptionError => {
                                EnqueueFanOutError::PushNotificationError
                            }
                        })?;
                    // TODO: It's currently not clear where we store the alert level.
                    let alert_level = 0;
                    push_token.send_notification(alert_level);
                }
            }
        }

        // Success!
        Ok(())
    }

    /// Update the fan out queue info with the given update message.
    pub(crate) fn apply_update(&mut self, update: QsFanOutQueueUpdate) {
        if let Some(pk) = update.qs_basic_queue_update.owner_public_key_option {
            self.owner_public_key = pk
        }
        if let Some(pk) = update.qs_basic_queue_update.owner_signature_key_option {
            self.owner_signature_key = pk
        }
        if let Some(push_token_option) = update.encrypted_push_token_option {
            self.encrypted_push_token_option = push_token_option
        }
        if let Some(auth_key) = update.encrypted_auth_key_option {
            self.encrypted_enqueue_auth_key = auth_key
        }
    }
}
