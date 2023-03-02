use crate::messages::intra_backend::DsFanOutMessage;

use super::{
    errors::QsEnqueueError, storage_provider_trait::QsStorageProvider, Qs,
    QueueIdDecryptionPrivateKey, WebsocketNotifier,
};

impl Qs {
    /// Enqueue the given message.
    #[tracing::instrument(skip_all, err)]
    pub async fn enqueue_message<S: QsStorageProvider, W: WebsocketNotifier>(
        storage_provider: &S,
        websocket_notifier: &W,
        message: DsFanOutMessage,
    ) -> Result<(), QsEnqueueError<S>> {
        // TODO: Load from storage provider (althouth this might turn into a symmetric key).
        let decryption_key: QueueIdDecryptionPrivateKey = todo!();
        // TODO: Decrypt queue config to yield the queue id.
        let queue_config = decryption_key.unseal_queue_config(&message.queue_config.sealed_config);
        let queue_id = &queue_config.queue_id;

        // Fetch the queue's info.
        let mut queue_info = storage_provider
            .load_queue_info(queue_id)
            .await
            .ok_or(QsEnqueueError::QueueNotFound)?;

        queue_info
            .enqueue(
                queue_id,
                storage_provider,
                websocket_notifier,
                message.payload,
                queue_config.push_token_key_option,
            )
            .await
            .map_err(QsEnqueueError::EnqueueFanOutError)
    }
}
