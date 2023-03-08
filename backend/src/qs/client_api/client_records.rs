use crate::{
    crypto::RatchetKey,
    ds::group_state::TimeStamp,
    messages::client_qs::{
        ClientRecordParams, CreateClientRecordParams, CreateClientRecordResponse,
        DeleteClientRecordParams, UpdateClientRecordParams,
    },
    qs::{
        client_record::QsClientRecord,
        errors::{QsCreateClientRecordError, QsGetClientError, QsUpdateClientRecordError},
        storage_provider_trait::QsStorageProvider,
        Qs,
    },
};

/*
- ENDPOINT_QS_CREATE_CLIENT_RECORD
- ENDPOINT_QS_UPDATE_CLIENT_RECORD
- ENDPOINT_QS_CLIENT_RECORD
- ENDPOINT_QS_DELETE_CLIENT_RECORD
*/

impl Qs {
    /// Create a new client record.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_create_client_record<S: QsStorageProvider>(
        &self,
        storage_provider: &S,
        params: CreateClientRecordParams,
    ) -> Result<CreateClientRecordResponse, QsCreateClientRecordError> {
        let CreateClientRecordParams {
            user_id,
            client_record_auth_key,
            queue_encryption_key,
            encrypted_key_packages,
            friendship_ear_key,
            encrypted_push_token,
        } = params;

        let seed = rand::random::<[u8; 32]>();

        // TODO: The ratchet key must be KEMed to the client using
        // `queue_encryption_key` and enqueued.
        let ratchet_key = RatchetKey::new(seed.to_vec());

        let client_record = QsClientRecord {
            user_id,
            encrypted_push_token,
            owner_public_key: queue_encryption_key,
            owner_signature_key: client_record_auth_key,
            current_ratchet_key: ratchet_key,
            activity_time: TimeStamp::now(),
        };

        // Get new client ID
        let client_id = storage_provider
            .create_client()
            .await
            .map_err(|_| QsCreateClientRecordError::StorageError)?;

        // Store client record
        storage_provider
            .store_client(&client_id, client_record)
            .await
            .map_err(|_| QsCreateClientRecordError::StorageError)?;

        // TODO: Validate the key packages
        let _ = friendship_ear_key;

        // Store key packages
        storage_provider
            .store_key_packages(&client_id, encrypted_key_packages)
            .await
            .map_err(|_| QsCreateClientRecordError::StorageError)?;

        let response = CreateClientRecordResponse { client_id };

        Ok(response)
    }

    /// Update a client record.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_update_client_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: UpdateClientRecordParams,
    ) -> Result<(), QsUpdateClientRecordError> {
        let UpdateClientRecordParams {
            client_id,
            client_record_auth_key,
            queue_encryption_key,
        } = params;

        let mut client_record = storage_provider
            .load_client(&client_id)
            .await
            .ok_or(QsUpdateClientRecordError::StorageError)?;

        client_record.update(client_record_auth_key, queue_encryption_key);

        storage_provider
            .store_client(&client_id, client_record)
            .await
            .map_err(|_| QsUpdateClientRecordError::StorageError)?;

        Ok(())
    }

    /// Get a client record.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_client_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: ClientRecordParams,
    ) -> Result<QsClientRecord, QsGetClientError> {
        let client_record = storage_provider
            .load_client(&params.client_id)
            .await
            .ok_or(QsGetClientError::StorageError)?;

        Ok(client_record)
    }

    /// Delete a client record.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_delete_client_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: DeleteClientRecordParams,
    ) -> Result<(), QsUpdateClientRecordError> {
        storage_provider
            .delete_client(&params.client_id)
            .await
            .map_err(|_| QsUpdateClientRecordError::StorageError)?;

        Ok(())
    }
}
