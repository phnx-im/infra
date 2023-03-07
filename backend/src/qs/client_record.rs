use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};
use tracing::instrument;

use crate::{
    crypto::{
        ear::{keys::PushTokenEarKey, DecryptionError, EarEncryptable},
        signatures::signable::Signature,
        signatures::{keys::QueueOwnerVerifyingKey, traits::SignatureVerificationError},
        RatchetKey, RatchetKeyUpdate, RatchetPublicKey,
    },
    ds::group_state::TimeStamp,
    messages::{client_ds::ClientToClientMsg, client_qs::EnqueuedMessage},
};

use super::{
    errors::{EnqueueError, QsCreateClientError},
    storage_provider_trait::QsStorageProvider,
    EncryptedPushToken, PushToken, QsClientId, WebsocketNotifier,
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

/// Info attached to a queue meant as a target for messages fanned out by a DS.
#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserialize, TlsSize)]
pub struct QsClientRecord {
    encrypted_push_token_option: Option<EncryptedPushToken>,
    owner_public_key: RatchetPublicKey,
    owner_signature_key: QueueOwnerVerifyingKey,
    current_ratchet_key: RatchetKey,
    activity_time: TimeStamp,
}

impl QsClientRecord {
    /// Creates a new queue and restursn the queue ID.
    pub(crate) async fn try_new<S: QsStorageProvider>(
        storage_provider: &S,
        encrypted_push_token_option: Option<EncryptedPushToken>,
        owner_public_key: RatchetPublicKey,
        owner_signature_key: QueueOwnerVerifyingKey,
        current_ratchet_key: RatchetKey,
    ) -> Result<(Self, QsClientId), QsCreateClientError<S>> {
        let fan_out_queue_info = Self {
            encrypted_push_token_option,
            owner_public_key,
            owner_signature_key,
            current_ratchet_key,
            activity_time: TimeStamp::now(),
        };
        let client_id = storage_provider
            .create_client(&fan_out_queue_info)
            .await
            .map_err(QsCreateClientError::StorageProviderError::<S>)?;
        Ok((fan_out_queue_info, client_id))
    }

    /// Update the client record.
    pub(crate) fn update(
        &mut self,
        client_record_auth_key: QueueOwnerVerifyingKey,
        queue_encryption_key: RatchetPublicKey,
    ) {
        self.owner_signature_key = client_record_auth_key;
        self.owner_public_key = queue_encryption_key;
        self.activity_time = TimeStamp::now();
    }

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

    /// Put a message into the queue.
    pub(crate) async fn enqueue<S: QsStorageProvider, W: WebsocketNotifier>(
        &mut self,
        client_id: &QsClientId,
        storage_provider: &S,
        websocket_notifier: &W,
        msg: ClientToClientMsg,
        push_token_key_option: Option<PushTokenEarKey>,
    ) -> Result<(), EnqueueError<S>> {
        // Serialize the message so that we can put it in the queue.
        // TODO: The message should be serialized differently, using a struct
        // with the sequence number
        let message_bytes = msg.assisted_message;

        // Encrypt the message under the current ratchet key.
        let encrypted_message = self.current_ratchet_key.encrypt(&message_bytes);

        // Ratchet the current ratchet key forward.
        let _ratchet_key_update = self.current_ratchet_key.ratchet_forward();

        tracing::trace!("Enqueueing message in storage provider");
        storage_provider
            .enqueue(client_id, encrypted_message)
            .await
            .map_err(EnqueueError::StorageProviderError::<S>)?;

        // Try to send a notification over the websocket, otherwise use push tokens if available
        if websocket_notifier.notify(client_id).await.is_err() {
            // Send a push notification under the following conditions:
            // - there is a push token associated with the queue
            // - there is a push token decryption key
            // - the decryption is successful
            if let Some(ref encrypted_push_token) = self.encrypted_push_token_option {
                if let Some(ref ear_key) = push_token_key_option {
                    let push_token =
                        PushToken::decrypt(ear_key, encrypted_push_token).map_err(|e| match e {
                            DecryptionError::DecryptionError => EnqueueError::PushNotificationError,
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
}
