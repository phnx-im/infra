// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
        storage_provider: &S,
        params: CreateUserRecordParams,
    ) -> Result<CreateUserRecordResponse, QsCreateUserError> {
        let CreateUserRecordParams {
            user_record_auth_key,
            friendship_token,
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
            initial_ratchet_secret,
        } = params;

        let user_record = QsUserRecord::new(user_record_auth_key, friendship_token);

        let user_id = storage_provider
            .create_user(user_record)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                QsCreateUserError::StorageError
            })?;

        let create_client_params = CreateClientRecordParams {
            sender: user_id.clone(),
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
            initial_ratchet_secret,
        };

        let CreateClientRecordResponse { client_id } =
            Self::qs_create_client_record(storage_provider, create_client_params)
                .await
                .map_err(|_| QsCreateUserError::StorageError)?;

        let response = CreateUserRecordResponse { user_id, client_id };

        Ok(response)
    }

    /// Update a user record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_update_user_record<S: QsStorageProvider>(
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
        Ok(())
    }

    /// Delete a user record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_delete_user_record<S: QsStorageProvider>(
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
