use crate::{
    messages::client_qs::{DequeueMessagesParams, DequeueMessagesResponse},
    qs::errors::QsFetchError,
};

use super::{storage_provider_trait::QsStorageProvider, Qs};

pub(crate) mod client_records;
pub(crate) mod key_packages;
pub(crate) mod user_records;

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
    /// Retrieve messages the given number of messages, starting with
    /// `sequence_number_start` from the queue with the given id and delete any
    /// messages older than the given sequence number start.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_dequeue_messages<S: QsStorageProvider>(
        storage_provider: &S,
        params: DequeueMessagesParams,
    ) -> Result<DequeueMessagesResponse, QsFetchError> {
        let DequeueMessagesParams {
            client_id,
            sequence_number_start,
            max_message_number,
        } = params;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        tracing::trace!("Reading and deleting messages from storage provider");
        let (messages, remaining_messages_number) = storage_provider
            .read_and_delete(&client_id, sequence_number_start, max_message_number)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsFetchError::StorageError
            })?;

        let response = DequeueMessagesResponse {
            messages,
            remaining_messages_number,
        };

        Ok(response)
    }
}
