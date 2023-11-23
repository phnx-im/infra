// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::qs::{QsCreateClientRecordError, QsUpdateClientRecordError},
    messages::client_qs::{
        CreateClientRecordParams, CreateClientRecordResponse, DeleteClientRecordParams,
        UpdateClientRecordParams,
    },
    time::TimeStamp,
};

use crate::qs::{client_record::QsClientRecord, storage_provider_trait::QsStorageProvider, Qs};

impl Qs {
    /// Create a new client record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_create_client_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: CreateClientRecordParams,
    ) -> Result<CreateClientRecordResponse, QsCreateClientRecordError> {
        let CreateClientRecordParams {
            sender,
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
            initial_ratchet_secret,
        } = params;

        let client_record = QsClientRecord {
            user_id: sender,
            encrypted_push_token,
            owner_public_key: queue_encryption_key,
            owner_signature_key: client_record_auth_key,
            current_ratchet_key: initial_ratchet_secret
                .try_into()
                .map_err(|_| QsCreateClientRecordError::LibraryError)?,
            activity_time: TimeStamp::now(),
        };

        // Get new client ID
        let client_id = storage_provider
            .create_client(client_record)
            .await
            .map_err(|_| QsCreateClientRecordError::StorageError)?;

        let response = CreateClientRecordResponse { client_id };

        Ok(response)
    }

    /// Update a client record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_update_client_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: UpdateClientRecordParams,
    ) -> Result<(), QsUpdateClientRecordError> {
        let UpdateClientRecordParams {
            sender,
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
        } = params;

        // TODO: It would be better to do this in an atomic transaction within
        // the storage provider

        let mut client_record = storage_provider
            .load_client(&sender)
            .await
            .ok_or(QsUpdateClientRecordError::StorageError)?;

        client_record.update(
            client_record_auth_key,
            queue_encryption_key,
            encrypted_push_token,
        );

        storage_provider
            .store_client(&sender, client_record)
            .await
            .map_err(|_| QsUpdateClientRecordError::StorageError)?;

        Ok(())
    }

    /// Delete a client record.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_delete_client_record<S: QsStorageProvider>(
        storage_provider: &S,
        params: DeleteClientRecordParams,
    ) -> Result<(), QsUpdateClientRecordError> {
        storage_provider
            .delete_client(&params.sender)
            .await
            .map_err(|_| QsUpdateClientRecordError::StorageError)?;

        Ok(())
    }
}
