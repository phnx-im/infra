use crate::messages::client_backend::{QsCreateQueueParams, QsCreateQueueParamsTBM};

use super::{errors::QsCreateQueueError, storage_provider_trait::QsStorageProvider, Qs};

impl Qs {
    /// Create a new, empty queue with the given info.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_create_queue<S: QsStorageProvider>(
        storage_provider: &S,
        params: QsCreateQueueParams,
    ) -> Result<(), QsCreateQueueError> {
        tracing::trace!("Verifying signature",);
        let QsCreateQueueParams { payload, signature } = params;
        let QsCreateQueueParamsTBM {
            queue_id,
            queue_info,
        } = payload;

        // Authenticate the message to make sure that the sender really does own
        // the signature key.
        queue_info
            .verify_against_owner_key(&signature)
            .map_err(|_| QsCreateQueueError::InvalidSignature)?;

        tracing::trace!("Storing new queue in storage provider");
        storage_provider
            .create_queue(&queue_id, queue_info)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsCreateQueueError::StorageError
            })
    }
}
