use crate::{
    crypto::RatchetKey,
    messages::client_qs::{
        ClientRecordParams, CreateClientRecordParams, CreateClientRecordResponse,
        DeleteClientRecordParams, UpdateClientRecordParams,
    },
    qs::{
        client_record::QsClientRecord,
        errors::{QsCreateClientRecordError, QsUpdateClientRecordError},
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
        storage_provider: &S,
        params: CreateClientRecordParams,
    ) -> Result<CreateClientRecordResponse, QsCreateClientRecordError> {
        let CreateClientRecordParams {
            client_record_auth_key,
            queue_encryption_key,
            key_packages,
            encrypted_push_token,
        } = params;

        let seed = rand::random::<[u8; 32]>();

        // TODO: The ratchet key must be KEMed to the client using
        // `queue_encryption_key`.
        let ratchet_key = RatchetKey::new(seed.to_vec());

        let (_client_record, client_id) = QsClientRecord::try_new(
            storage_provider,
            encrypted_push_token,
            queue_encryption_key,
            client_record_auth_key,
            ratchet_key,
        )
        .await
        .map_err(|_| QsCreateClientRecordError::StorageError)?;

        // TODO: STore the key packages in the storage provider.

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
    ) -> Result<QsClientRecord, QsUpdateClientRecordError> {
        let client_record = storage_provider
            .load_client(&params.client_id)
            .await
            .ok_or(QsUpdateClientRecordError::StorageError)?;

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

        // TODO: Delete the key packages
        // TODO: Should we check the owning user,
        // and delete is if it doesn't have any clients anymore?

        Ok(())
    }
}
