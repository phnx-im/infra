// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    auth_service::{credentials::ClientCredential, errors::*, storage_provider_trait::*, *},
    messages::client_as::*,
};

impl AuthService {
    pub async fn as_init_user_registration<S: AsStorageProvider, E: AsEphemeralStorageProvider>(
        &self,
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: InitUserRegistrationParams,
    ) -> Result<InitUserRegistrationResponse, InitUserRegistrationError> {
        let InitUserRegistrationParams {
            auth_method,
            client_csr,
            opaque_registration_request,
        } = params;

        // Check if a user entry with the name given in the client_csr already exists
        if storage_provider
            .load_user(&client_csr.identity().username())
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitUserRegistrationError::StorageError
            })?
            .is_some()
        {
            return Err(InitUserRegistrationError::UserAlreadyExists);
        }

        // Validate the client_csr
        if !client_csr.validate() {
            return Err(InitUserRegistrationError::InvalidCsr);
        }

        // Sign the CSR
        let client_credential = ClientCredential::new_from_csr(client_csr);

        // Store the client_credential in the ephemeral DB
        ephemeral_storage_provider
            .store_credential(client_credential.identity(), &client_credential)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitUserRegistrationError::StorageError
            })?;

        /*
        TODO OPAQUE:
             - perform the first (server side) step in the OPAQUE registration handshake
             - return the ClientCredential to the client along with the OPAQUE response.
         */

        let opaque_registration_response = OpaqueRegistrationResponse {};

        let response = InitUserRegistrationResponse {
            client_credential,
            opaque_registration_response,
        };

        Ok(response)
    }

    pub async fn as_finish_user_registration<
        S: AsStorageProvider,
        E: AsEphemeralStorageProvider,
    >(
        &self,
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: FinishUserRegistrationParams,
    ) -> Result<FinishUserRegistrationResponse, FinishUserRegistrationError> {
        let FinishUserRegistrationParams {
            auth_method,
            user_name,
            queue_encryption_key,
            initial_ratchet_key,
            connection_key_packages,
            opaque_registration_record,
        } = params;

        let Client2FaAuth {
            client_id,
            password,
        } = auth_method;

        // Look up the initial client's ClientCredential in the ephemeral DB based on the user_name
        let client_credential = ephemeral_storage_provider
            .load_credential(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?
            .ok_or(FinishUserRegistrationError::ClientCredentialNotFound)?;

        // Authenticate the request using the signature key in the
        // ClientCredential

        // TODO: This is tricky, since we cannot do this ahead
        // of time, since the client certificate is only in the ephemeral DB.

        // Create the user entry with the information given in the request
        let user_record = storage_provider
            .create_user(&client_id.username())
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        // Create the initial client entry

        let client_record = AsClientRecord {
            queue_encryption_key,
            ratchet_key: initial_ratchet_key,
            activity_time: TimeStamp::now(),
        };

        storage_provider
            .store_client(&client_id, &client_record)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        // Delete the entry in the ephemeral OPAQUE DB
        ephemeral_storage_provider
            .delete_credential(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        let response = FinishUserRegistrationResponse {};

        Ok(response)
    }

    pub async fn as_delete_user<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: DeleteUserParams,
    ) -> Result<DeleteUserResponse, DeleteUserError> {
        let DeleteUserParams {
            auth_method,
            user_name,
        } = params;

        let Client2FaAuth {
            client_id,
            password,
        } = auth_method;

        // Delete the user
        storage_provider
            .delete_user(&client_id.username())
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                DeleteUserError::StorageError
            })?;
        let response = DeleteUserResponse {};

        Ok(response)
    }
}
