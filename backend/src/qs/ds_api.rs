use crate::messages::intra_backend::DsFanOutMessage;

use super::{
    errors::QsEnqueueError, storage_provider_trait::QsStorageProvider, ClientIdEncryptionPublicKey,
    Qs, WebsocketNotifier,
};

impl Qs {
    /// Enqueue the given message. This endpoint is called by the local DS
    /// during a fanout operation. This endpoint does not necessairly return
    /// quickly. It can attempt to do the full fanout and return potential
    /// failed transmissions to the DS.
    ///
    /// This endpoint is used for enqueining
    /// messages in both local and remote queues, depending on the FQDN of the
    /// client. For now, only local queues are supported.
    #[tracing::instrument(skip_all, err)]
    pub async fn enqueue_message<S: QsStorageProvider, W: WebsocketNotifier>(
        &self,
        storage_provider: &S,
        websocket_notifier: &W,
        message: DsFanOutMessage,
    ) -> Result<(), QsEnqueueError<S>> {
        let decryption_key = &self.client_id_private_key;
        let client_config =
            decryption_key.unseal_client_config(&message.client_reference.sealed_reference)?;

        // Fetch the client record.
        let mut client_record = storage_provider
            .load_client(&client_config.client_id)
            .await
            .ok_or(QsEnqueueError::QueueNotFound)?;

        Ok(client_record
            .enqueue(
                &client_config.client_id,
                storage_provider,
                websocket_notifier,
                message.payload,
                client_config.push_token_ear_key,
            )
            .await?)

        // TODO: client now has new ratchet key, store it in the storage
        // provider.
    }

    /// Get the client config encryption key.
    #[tracing::instrument(skip_all)]
    pub async fn qs_client_config_encryption_key(&self) -> ClientIdEncryptionPublicKey {
        self.client_id_public_key.clone()
    }
}
