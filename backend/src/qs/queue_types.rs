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
    messages::client_backend::{ClientToClientMsg, QsFanOutQueueUpdate, QsQueueUpdate},
};

use super::{
    errors::{EnqueueBasicError, EnqueueFanOutError},
    storage_provider_trait::QsStorageProvider,
    EncryptedPushToken, PushToken, QueueId, WebsocketNotifier,
};

#[derive(Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub ciphertext: Vec<u8>,
}

/// An enum defining the different kind of messages that are stored in an QS
/// queue.
/// TODO: This needs a codec that allows decoding to the proper type.
#[derive(Serialize, Deserialize)]
pub(super) enum QsQueueMessage {
    RatchetKeyUpdate(RatchetKeyUpdate),
    EncryptedMessage(EncryptedMessage),
}

/// Info attached to a basic QS queue.
#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct BasicQueueInfo {
    owner_public_key: RatchetPublicKey,
    owner_signature_key: QueueOwnerVerificationKey,
    current_ratchet_key: RatchetKey,
    // Encrypted key that authenticates entities that want to delete the queue.
    encrypted_delete_auth_key: QueueDeletionAuthKeyCtxt,
}

impl BasicQueueInfo {
    /// Inject fresh entropy into the current ratchet key to achieve PCS for the
    /// queue encryption.
    /// TODO: This should be called regularly by the QS. Either just
    /// periodically, or after a set number of messages in each queue.
    pub(crate) async fn update_ratchet_key<S: QsStorageProvider>(
        &mut self,
        storage_provider: &mut S,
        queue_id: &QueueId,
    ) -> Result<(), S::EnqueueError> {
        let ratchet_key_update = self.current_ratchet_key.update();
        let encrypted_ratchet_key_update = self
            .owner_public_key
            .encrypt_ratchet_key_update(&ratchet_key_update);

        storage_provider
            .enqueue(queue_id, encrypted_ratchet_key_update)
            .await
    }

    /// Encrypt and enqueue the given message.
    #[instrument(level = "trace", skip_all, err, fields(
        queue_id = %queue_id,
    ))]
    async fn enqueue_message<S: QsStorageProvider>(
        &mut self,
        queue_id: &QueueId,
        storage_provider: &S,
        message: Vec<u8>,
    ) -> Result<(), EnqueueBasicError<S>> {
        // Encrypt the message under the current ratchet key.
        let encrypted_message = self.current_ratchet_key.encrypt(&message);

        // Ratchet the current ratchet key forward.
        let _ratchet_key_update = self.current_ratchet_key.ratchet_forward();

        tracing::trace!("Enqueueing message in storage provider");
        storage_provider
            .enqueue(queue_id, encrypted_message)
            .await
            .map_err(EnqueueBasicError::StorageProviderError::<S>)?;

        Ok(())
    }

    pub(crate) fn encrypted_delete_auth_key(&self) -> &QueueDeletionAuthKeyCtxt {
        &self.encrypted_delete_auth_key
    }
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
    // Info of the basic queue underlying the fan-out queue.
    basic_queue_info: BasicQueueInfo,
}

/// A direct queue doesn't have any info beside the info of internal
/// basic queue.
/// TODO: We likely won't need this queue anymore.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectQueueInfo {
    info: BasicQueueInfo,
}

impl FanOutQueueInfo {
    /// Verify the request against the signature key of the queue owner. Returns
    /// an error if the authentication fails.
    #[instrument(level = "trace", skip_all, err)]
    pub(crate) fn verify_against_owner_key(
        &self,
        signature: &Signature,
    ) -> Result<(), SignatureVerificationError> {
        // TODO: This should verify a QsAuthToken instead of a signature over a
        // request hash.
        //self.basic_queue_info
        //    .owner_signature_key
        //    .verify(request_hash, signature)
        //    .map_err(|_| VerificationError::VerificationFailure)
        todo!()
    }

    pub(crate) fn basic_queue_info(&self) -> &BasicQueueInfo {
        &self.basic_queue_info
    }
}

impl DirectQueueInfo {
    ///// Take the given message and put it into the queue.
    //#[instrument(level = "trace", skip_all, err)]
    //pub(crate) async fn enqueue<S: QsStorageProvider>(
    //    &mut self,
    //    storage_provider: &S,
    //    msg: &QsDirectMessage,
    //) -> Result<(), EnqueueDirectError<S>> {
    //    let queue_id = &msg.queue_id;
    //    // Serialize the message so that we can put it in the queue.
    //    let message_bytes =
    //    // serialization shouldn't fail
    //        serde_json::to_vec(&msg.welcome).map_err(|_| EnqueueDirectError::LibraryError)?;

    //    // Enqueue the message. The queue deals with the ratchet encryption for EAR
    //    self.info
    //        .enqueue_message(queue_id, storage_provider, message_bytes)
    //        .await
    //        .map_err(EnqueueDirectError::EnqueuingError)?;

    //    // Success!
    //    Ok(())
    //}

    /// Update the fan out queue info with the given update message.
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn apply_update(&mut self, update: QsQueueUpdate) {
        if let Some(pk) = update.owner_public_key_option {
            self.info.owner_public_key = pk
        }
        if let Some(pk) = update.owner_signature_key_option {
            self.info.owner_signature_key = pk
        }
    }
}

impl FanOutQueueInfo {
    /// Put a message into the queue.
    #[instrument(level = "trace", skip_all, err)]
    pub(crate) async fn enqueue<S: QsStorageProvider, W: WebsocketNotifier>(
        &mut self,
        queue_id: &QueueId,
        storage_provider: &S,
        websocket_notifier: &W,
        msg: ClientToClientMsg,
        push_token_key_option: Option<PushTokenEarKey>,
    ) -> Result<(), EnqueueFanOutError<S>> {
        // Serialize the message so that we can put it in the queue.
        let message_bytes =
        // serialization shouldn't fail
        msg.tls_serialize_detached().map_err(|_| EnqueueFanOutError::LibraryError)?;

        // Enqueue the message. The queue deals with the ratchet encryption for EAR
        self.basic_queue_info
            .enqueue_message(&queue_id, storage_provider, message_bytes)
            .await
            .map_err(EnqueueFanOutError::EnqueuingError)?;

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
                    let alert_level = todo!();
                    push_token.send_notification(alert_level);
                }
            }
        }

        // Success!
        Ok(())
    }

    /// Update the fan out queue info with the given update message.
    #[instrument(level = "trace", skip_all)]
    pub(crate) fn apply_update(&mut self, update: QsFanOutQueueUpdate) {
        if let Some(pk) = update.qs_basic_queue_update.owner_public_key_option {
            self.basic_queue_info.owner_public_key = pk
        }
        if let Some(pk) = update.qs_basic_queue_update.owner_signature_key_option {
            self.basic_queue_info.owner_signature_key = pk
        }
        if let Some(push_token_option) = update.encrypted_push_token_option {
            self.encrypted_push_token_option = push_token_option
        }
        if let Some(auth_key) = update.encrypted_auth_key_option {
            self.encrypted_enqueue_auth_key = auth_key
        }
    }
}
