use crate::{
    crypto::{
        ear::{DecryptionError, EarEncryptable},
        mac::{keys::QueueDeletionAuthKey, MacVerificationError, TagVerifiable},
    },
    messages::client_qs::{
        EnqueuedMessage, QsDeleteQueueRequest, QsFetchMessageParamsTBS, QsFetchMessagesParams,
        QsUpdateQueueInfoParams, QsUpdateQueueInfoParamsTBS,
    },
    qs::errors::{QsDeleteQueueError, QsFetchError, QsUpdateQueueError},
};

use super::{storage_provider_trait::QsStorageProvider, Qs};

/*
Endpoints:
 - ENDPOINT_QS_QC_ENCRYPTION_KEY
 - ENDPOINT_QS_CREATE_USER_RECORD
 - ENDPOINT_QS_UPDATE_USER_RECORD
 - ENDPOINT_QS_USER_RECORD
 - ENDPOINT_QS_DELETE_USER_RECORD
 - ENDPOINT_QS_CREATE_CLIENT_RECORD
 - ENDPOINT_QS_UPDATE_CLIENT_RECORD
 - ENDPOINT_QS_CLIENT_RECORD
 - ENDPOINT_QS_DELETE_CLIENT_RECORD
 - ENDPOINT_QS_PUBLISH_KEY_PACKAGES
 - ENDPOINT_QS_CLIENT_KEY_PACKAGE
 - ENDPOINT_QS_KEY_PACKAGE_BATCH
 - ENDPOINT_QS_DEQUEUE_MESSAGES
 - ENDPOINT_QS_WS
*/

impl Qs {
    /// Update the info of a given queue. Requires a valid signature by the
    /// owner of the queue.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_update_queue_info<S: QsStorageProvider>(
        storage_provider: &S,
        params: QsUpdateQueueInfoParams,
    ) -> Result<(), QsUpdateQueueError> {
        let QsUpdateQueueInfoParams { payload, signature } = params;
        let QsUpdateQueueInfoParamsTBS {
            queue_id,
            info_update,
        } = payload;

        tracing::trace!("Loading current queue info from storage provider");
        let mut queue_info = storage_provider
            .load_queue_info(&queue_id)
            .await
            .ok_or(QsUpdateQueueError::QueueNotFound)?;

        // Authenticate the owner of the queue.
        queue_info
            .verify_against_owner_key(&signature)
            .map_err(|_| QsUpdateQueueError::WrongQueueType)?;

        // Apply the update depending on the queue type, or throw an error if
        // the update and the queue type don't match.
        queue_info.apply_update(info_update);

        tracing::trace!("Updating queue info in storage provider");
        storage_provider
            .save_queue_info(&queue_id, queue_info)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsUpdateQueueError::StorageError
            })
    }

    /// Retrieve messages the given number of messages, starting with
    /// `sequence_number_start` from the queue with the given id and delete any
    /// messages older than the given sequence number start.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_fetch_messages<S: QsStorageProvider>(
        storage_provider: &S,
        params: QsFetchMessagesParams,
    ) -> Result<(Vec<EnqueuedMessage>, u64), QsFetchError> {
        let QsFetchMessagesParams { payload, signature } = params;
        let QsFetchMessageParamsTBS {
            queue_id,
            sequence_number_start,
            max_messages,
        } = payload;

        // Fetch the queue that's registered in the alias.
        tracing::trace!("Loading queue info from storage provider");
        let queue_info = storage_provider
            .load_queue_info(&queue_id)
            .await
            .ok_or(QsFetchError::QueueNotFound)?;

        // Authenticate the owner of the queue.
        queue_info
            .verify_against_owner_key(&signature)
            .map_err(|_| QsFetchError::InvalidSignature)?;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        tracing::trace!("Reading and deleting messages from storage provider");
        storage_provider
            .read_and_delete(&queue_id, sequence_number_start, max_messages)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsFetchError::StorageError
            })
    }

    /// Delete the queue with the given queue id.
    #[tracing::instrument(
        skip_all,
        fields(
            queue_id = %request.payload.queue_id,
        ),
        err
    )]
    pub async fn qs_delete_queue<S: QsStorageProvider>(
        storage_provider: &S,
        request: QsDeleteQueueRequest,
    ) -> Result<(), QsDeleteQueueError> {
        // Fetch the queue's info.
        let queue_info = storage_provider
            .load_queue_info(&request.payload.queue_id)
            .await
            .ok_or(QsDeleteQueueError::QueueNotFound)?;

        // Authenticate the message.
        let delete_auth_key = QueueDeletionAuthKey::decrypt(
            &request.payload.auth_token_key,
            queue_info.encrypted_delete_auth_key(),
        )
        .map_err(|e| match e {
            DecryptionError::DecryptionError => QsDeleteQueueError::AuthKeyDecryptionFailure,
        })?;

        let verified_payload = request.verify(&delete_auth_key).map_err(|e| match e {
            MacVerificationError::VerificationFailure => QsDeleteQueueError::AuthenticationFailure,
            MacVerificationError::LibraryError => QsDeleteQueueError::LibraryError,
        })?;

        tracing::trace!("Deleting queue from storage provider");
        storage_provider
            .delete_queue(&verified_payload.queue_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsDeleteQueueError::StorageError
            })?;

        Ok(())
    }
}
