// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    auth_service::{credentials::*, errors::*, storage_provider_trait::*, *},
    messages::client_as::*,
};

impl AuthService {
    pub async fn as_init_client_addition<S: AsStorageProvider, E: AsEphemeralStorageProvider>(
        &self,
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: InitiateClientAdditionParams,
    ) -> Result<InitClientAdditionResponse, InitClientAdditionError> {
        let InitiateClientAdditionParams {
            auth_method,
            client_csr,
            opaque_ke1,
        } = params;

        // Check if a client entry with the name given in the client_csr already exists for the user
        if storage_provider
            .load_client(&client_csr.identity())
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitClientAdditionError::StorageError
            })?
            .is_some()
        {
            return Err(InitClientAdditionError::ClientAlreadyExists);
        }

        // Validate the client_csr
        if !client_csr.validate() {
            return Err(InitClientAdditionError::InvalidCsr);
        }

        // Sign the CSR
        let client_credential = ClientCredential::new_from_csr(client_csr);

        // Store the client_credential in the ephemeral DB
        ephemeral_storage_provider
            .store_credential(client_credential.identity(), &client_credential)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitClientAdditionError::StorageError
            })?;

        /*
        TODO OPAQUE:
             - perform the first (server side) step in the OPAQUE registration handshake
             - return the ClientCredential to the client along with the OPAQUE response.
         */

        let opaque_ke2 = OpaqueKe2 {};

        let response = InitClientAdditionResponse {
            client_credential,
            opaque_ke2,
        };

        Ok(response)
    }

    pub async fn as_finish_client_addition<S: AsStorageProvider, E: AsEphemeralStorageProvider>(
        &self,
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: FinishClientAdditionParams,
    ) -> Result<FinishClientAdditionResponse, FinishUserRegistrationError> {
        let FinishClientAdditionParams {
            auth_method,
            client_id,
            queue_encryption_key,
            initial_ratchet_key,
            connection_key_package,
            opaque_ke3,
        } = params;

        let Client2FaAuth {
            client_id,
            password,
        } = auth_method;

        // Look up the initial client's ClientCredential in the ephemeral DB based on the client_id
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

        // Create the new client entry
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

        let response = FinishClientAdditionResponse {};

        Ok(response)
    }

    pub async fn as_delete_client<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: DeleteClientParams,
    ) -> Result<DeleteClientResponse, DeleteClientError> {
        let DeleteClientParams {
            auth_method,
            client_id,
        } = params;

        // Delete the client
        storage_provider
            .delete_client(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                DeleteClientError::StorageError
            })?;
        let response = DeleteClientResponse {};

        Ok(response)
    }

    pub async fn as_dequeue_messages<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: DequeueMessagesParams,
    ) -> Result<DequeueMessagesResponse, AsDequeueError> {
        let DequeueMessagesParams {
            auth_method,
            sender,
            sequence_number_start,
            max_message_number,
        } = params;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        tracing::trace!("Reading and deleting messages from storage provider");
        let (messages, remaining_messages_number) = storage_provider
            .read_and_delete(&sender, sequence_number_start, max_message_number)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                AsDequeueError::StorageError
            })?;

        let response = DequeueMessagesResponse {
            messages,
            remaining_messages_number,
        };

        Ok(response)
    }
}
