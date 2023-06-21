// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    auth_service::{errors::*, storage_provider_trait::AsStorageProvider, AuthService},
    messages::client_as::*,
};

impl AuthService {
    pub(crate) async fn as_user_clients<S: AsStorageProvider>(
        storage_provider: &S,
        params: UserClientsParams,
    ) -> Result<UserClientsResponse, UserClientsError> {
        let UserClientsParams { user_name } = params;

        // Look up the user entry in the DB
        let client_credentials = storage_provider.client_credentials(&user_name).await;

        let response = UserClientsResponse { client_credentials };

        Ok(response)
    }

    pub async fn as_user_key_package<S: AsStorageProvider>(
        storage_provider: &S,
        params: UserConnectionPackagesParams,
    ) -> Result<UserConnectionPackagesResponse, UserKeyPackagesError> {
        let UserConnectionPackagesParams { user_name } = params;

        let key_packages = storage_provider
            .load_user_connection_packages(&user_name)
            .await
            .map_err(|_| UserKeyPackagesError::StorageError)?;

        let response = UserConnectionPackagesResponse { key_packages };
        Ok(response)
    }

    pub(crate) async fn as_enqueue_message<S: AsStorageProvider>(
        storage_provider: &S,
        params: EnqueueMessageParams,
    ) -> Result<(), EnqueueMessageError> {
        let EnqueueMessageParams {
            client_id,
            connection_establishment_ctxt,
        } = params;

        // Fetch the client record.
        let mut client_record = storage_provider
            .load_client(&client_id)
            .await
            .ok_or(EnqueueMessageError::ClientNotFound)?;

        let payload = connection_establishment_ctxt
            .try_into()
            .map_err(|_| EnqueueMessageError::LibraryError)?;

        let queue_message = client_record
            .ratchet_key
            .encrypt(payload)
            .map_err(|_| EnqueueMessageError::LibraryError)?;

        // TODO: Future work: PCS

        tracing::trace!("Enqueueing message in storage provider");
        storage_provider
            .enqueue(&client_id, queue_message)
            .await
            .map_err(|_| EnqueueMessageError::StorageError)?;

        // Store the changed client record.
        storage_provider
            .store_client(&client_id, &client_record)
            .await
            .map_err(|_| EnqueueMessageError::StorageError)?;

        Ok(())
    }

    pub(crate) async fn as_credentials<S: AsStorageProvider>(
        storage_provider: &S,
        params: AsCredentialsParams,
    ) -> Result<AsCredentialsResponse, AsCredentialsError> {
        let (as_credentials, as_intermediate_credentials, revoked_credentials) = storage_provider
            .load_as_credentials()
            .await
            .map_err(|_| AsCredentialsError::StorageError)?;
        Ok(AsCredentialsResponse {
            as_credentials,
            as_intermediate_credentials,
            revoked_credentials,
        })
    }
}
