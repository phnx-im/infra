use crate::{
    messages::client_qs::{
        CreateClientRecordParams, CreateClientRecordResponse, CreateUserRecordParams,
        CreateUserRecordResponse, DeleteUserRecordParams, UpdateUserRecordParams,
    },
    qs::{
        errors::{QsCreateUserError, QsDeleteUserError, QsUpdateUserError},
        storage_provider_trait::QsStorageProvider,
        user_record::QsUserRecord,
        Qs,
    },
};

impl Qs {
    /// Update the info of a given queue. Requires a valid signature by the
    /// owner of the queue.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_create_user_record<S: QsStorageProvider>(
        &self,
        storage_provider: &S,
        params: CreateUserRecordParams,
    ) -> Result<CreateUserRecordResponse, QsCreateUserError> {
        let CreateUserRecordParams {
            user_record_auth_key,
            friendship_token,
            client_record_auth_key,
            queue_encryption_key,
            add_packages,
            friendship_ear_key,
            encrypted_push_token,
            initial_ratchet_key,
        } = params;

        let user_id = storage_provider.create_user().await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            QsCreateUserError::StorageError
        })?;

        let create_client_params = CreateClientRecordParams {
            sender: user_id.clone(),
            client_record_auth_key,
            queue_encryption_key,
            add_packages,
            friendship_ear_key,
            encrypted_push_token,
            initial_ratchet_key,
        };

        let CreateClientRecordResponse { client_id } = self
            .qs_create_client_record(storage_provider, create_client_params)
            .await
            .map_err(|_| QsCreateUserError::StorageError)?;

        let user_record = QsUserRecord::new(user_record_auth_key, friendship_token);

        tracing::trace!("Storing QsUserProfile in storage provider");
        storage_provider
            .store_user(&user_id, user_record)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsCreateUserError::StorageError
            })?;

        let response = CreateUserRecordResponse { user_id, client_id };

        Ok(response)
    }

    /// Update a user record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_update_user_record<S: QsStorageProvider>(
        &self,
        storage_provider: &S,
        params: UpdateUserRecordParams,
    ) -> Result<(), QsUpdateUserError> {
        let UpdateUserRecordParams {
            sender,
            user_record_auth_key,
            friendship_token,
        } = params;

        let mut user_record = storage_provider
            .load_user(&sender)
            .await
            .ok_or(QsUpdateUserError::StorageError)?;

        user_record.update(user_record_auth_key, friendship_token);

        storage_provider
            .store_user(&sender, user_record)
            .await
            .map_err(|_| QsUpdateUserError::StorageError)?;
        todo!()
    }

    /// Delete a user record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_delete_user_record<S: QsStorageProvider>(
        &self,
        storage_provider: &S,
        params: DeleteUserRecordParams,
    ) -> Result<(), QsDeleteUserError> {
        let DeleteUserRecordParams { sender } = params;

        storage_provider
            .delete_user(&sender)
            .await
            .map_err(|_| QsDeleteUserError::StorageError)?;

        Ok(())
    }
}
