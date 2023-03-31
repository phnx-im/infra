// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::convert::Infallible;

use crate::{
    auth_service::{errors::*, storage_provider_trait::AsStorageProvider, AuthService},
    messages::client_as::*,
};

impl AuthService {
    pub async fn as_user_clients<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: UserClientsParams,
    ) -> Result<UserClientsResponse, UserClientsError> {
        let UserClientsParams {
            auth_method,
            user_name,
        } = params;

        // Look up the user entry in the DB
        let client_credentials = storage_provider.client_credentials(&user_name).await;

        let response = UserClientsResponse { client_credentials };

        Ok(response)
    }

    pub async fn as_user_key_package<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: UserKeyPackagesParams,
    ) -> Result<UserKeyPackagesResponse, UserKeyPackagesError> {
        let UserKeyPackagesParams {
            auth_method,
            user_name,
        } = params;

        let key_packages = storage_provider
            .load_user_key_packages(&user_name)
            .await
            .map_err(|_| UserKeyPackagesError::StorageError)?;

        let response = UserKeyPackagesResponse { key_packages };
        Ok(response)
    }

    pub async fn as_enqueue_message<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: EnqueueMessageParams,
    ) -> Result<(), EnqueueMessageError> {
        let EnqueueMessageParams {
            auth_method,
            client_id,
            connection_establishment_ctxt,
        } = params;

        // Fetch the client record.
        let mut client_record = storage_provider
            .load_client(&client_id)
            .await
            .map_err(|_| EnqueueMessageError::StorageError)?
            .ok_or(EnqueueMessageError::ClientNotFound)?;

        let queue_message = client_record
            .ratchet_key
            .encrypt(connection_establishment_ctxt)
            .map_err(|_| EnqueueMessageError::LibraryError)?;

        // TODO: Store the new key.

        // TODO: Future work: PCS

        tracing::trace!("Enqueueing message in storage provider");
        storage_provider
            .enqueue(&client_id, queue_message)
            .await
            .map_err(|_| EnqueueMessageError::StorageError)?;

        // TODO: client now has new ratchet key, store it in the storage
        // provider.
        Ok(())
    }

    // TODO: Credentials
    pub async fn as_credentials(
        &self,
        params: AsCredentialsParams,
    ) -> Result<AsCredentialsResponse, Infallible> {
        todo!()
    }
}
