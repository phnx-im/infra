// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::auth_service::{
        AsCredentialsError, EnqueueMessageError, UserClientsError, UserConnectionPackagesError,
    },
    messages::client_as::{
        AsCredentialsParams, AsCredentialsResponse, EnqueueMessageParams, UserClientsParams,
        UserClientsResponse, UserConnectionPackagesParams, UserConnectionPackagesResponse,
    },
};

use crate::auth_service::{storage_provider_trait::AsStorageProvider, AuthService};

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

    pub async fn as_user_connection_packages<S: AsStorageProvider>(
        storage_provider: &S,
        params: UserConnectionPackagesParams,
    ) -> Result<UserConnectionPackagesResponse, UserConnectionPackagesError> {
        let UserConnectionPackagesParams { user_name } = params;

        let connection_packages = storage_provider
            .load_user_connection_packages(&user_name)
            .await
            .map_err(|e| {
                tracing::warn!(
                    "Failed to load connection packages due to storage error: {:?}",
                    e
                );
                UserConnectionPackagesError::StorageError
            })?;

        // If there are no connection packages, we have to conclude that there
        // is no user.
        if connection_packages.is_empty() {
            return Err(UserConnectionPackagesError::UnknownUser);
        }

        let response = UserConnectionPackagesResponse {
            key_packages: connection_packages,
        };
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
            .map_err(|e| {
                tracing::warn!("Failed to enqueue message: {:?}", e);
                EnqueueMessageError::StorageError
            })?;

        // Store the changed client record.
        storage_provider
            .store_client(&client_id, &client_record)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to store client record: {:?}", e);
                EnqueueMessageError::StorageError
            })?;

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
