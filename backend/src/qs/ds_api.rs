use crate::messages::intra_backend::DsFanOutMessage;

use super::{
    errors::QsEnqueueError, storage_provider_trait::QsStorageProvider, Qs, WebsocketNotifier,
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
        // TODO: Load from storage provider (althouth this might turn into a symmetric key).
        let decryption_key = &self.queue_id_private_key;
        // TODO: Decrypt queue config to yield the queue id.
        let client_config =
            decryption_key.unseal_queue_config(&message.client_reference.sealed_reference);

        // Fetch the queue's info.
        let mut client_record = storage_provider
            .load_client(&client_config.client_id)
            .await
            .ok_or(QsEnqueueError::QueueNotFound)?;

        client_record
            .enqueue(
                &client_config.client_id,
                storage_provider,
                websocket_notifier,
                message.payload,
                client_config.push_token_key_option,
            )
            .await
            .map_err(QsEnqueueError::EnqueueError)
    }
}
