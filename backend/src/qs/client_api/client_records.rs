use crate::{
    crypto::ear::EarEncryptable,
    ds::group_state::TimeStamp,
    messages::client_qs::{
        CreateClientRecordParams, CreateClientRecordResponse, DeleteClientRecordParams,
        UpdateClientRecordParams,
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
        &self,
        storage_provider: &S,
        params: CreateClientRecordParams,
    ) -> Result<CreateClientRecordResponse, QsCreateClientRecordError> {
        let CreateClientRecordParams {
            sender,
            client_record_auth_key,
            queue_encryption_key,
            key_packages,
            friendship_ear_key,
            encrypted_push_token,
            initial_ratchet_key,
        } = params;

        let client_record = QsClientRecord {
            user_id: sender,
            encrypted_push_token,
            owner_public_key: queue_encryption_key,
            owner_signature_key: client_record_auth_key,
            current_ratchet_key: initial_ratchet_key,
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

        let encrypted_key_packages = key_packages
            .into_iter()
            .map(|key_package| {
                key_package
                    .encrypt(&friendship_ear_key)
                    .map_err(|_| QsCreateClientRecordError::LibraryError)
            })
            .collect::<Result<Vec<_>, _>>()?;

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
            encrypted_push_token,
        } = params;

        // TODO: It would be better to do this in an atomic transaction within
        // the storage provider

        let mut client_record = storage_provider
            .load_client(&client_id)
            .await
            .ok_or(QsUpdateClientRecordError::StorageError)?;

        client_record.update(
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
        );

        storage_provider
            .store_client(&client_id, client_record)
            .await
            .map_err(|_| QsUpdateClientRecordError::StorageError)?;

        Ok(())
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
