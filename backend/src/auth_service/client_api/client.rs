// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::{ServerLogin, ServerLoginStartParameters};
use privacypass::Serialize;
use rand_chacha::rand_core::OsRng;

use crate::{
    auth_service::{credentials::*, errors::*, storage_provider_trait::*, *},
    crypto::signatures::signable::Signable,
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
            client_credential_payload,
            opaque_login_request,
        } = params;

        // Load the server setup from storage
        let server_setup = storage_provider.load_opaque_setup().await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            InitClientAdditionError::StorageError
        })?;

        // Load the user record from storage
        let user_name = client_credential_payload.identity().username();
        let password_file_option = storage_provider
            .load_user(&user_name)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitClientAdditionError::StorageError
            })?
            .map(|record| record.password_file);

        let server_login_result = ServerLogin::<OpaqueCiphersuite>::start(
            &mut OsRng,
            &server_setup,
            password_file_option,
            opaque_login_request.client_message,
            &user_name
                .tls_serialize_detached()
                .map_err(|_| InitClientAdditionError::LibraryError)?,
            // TODO: We probably want to specify a context, as well as a server
            // and client name here. For now, the default should be okay.
            ServerLoginStartParameters::default(),
        )
        .map_err(|e| {
            tracing::error!("Opaque startup failed with error {e:?}");
            InitClientAdditionError::OpaqueLoginFailed
        })?;

        let opaque_login_response = OpaqueLoginResponse {
            server_message: server_login_result.message,
        };

        // Check if a client entry with the name given in the client_csr already exists for the user
        let client_id_exists = storage_provider
            .load_client(&client_credential_payload.identity())
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitClientAdditionError::StorageError
            })?
            .is_some();

        if client_id_exists {
            return Err(InitClientAdditionError::ClientAlreadyExists);
        }

        // Validate the client credential payload
        if !client_credential_payload.validate() {
            return Err(InitClientAdditionError::InvalidCsr);
        }

        // Load the signature key from storage.
        let signing_key = storage_provider.load_signing_key().await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            InitClientAdditionError::StorageError
        })?;

        // Sign the credential
        let client_credential: ClientCredential = client_credential_payload
            .sign(&signing_key)
            .map_err(|_| InitClientAdditionError::LibraryError)?;

        // Store the client_credential in the ephemeral DB
        ephemeral_storage_provider
            .store_login_state(
                client_credential.identity(),
                &client_credential,
                &server_login_result.state,
            )
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitClientAdditionError::StorageError
            })?;

        let response = InitClientAdditionResponse {
            client_credential,
            opaque_login_response,
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
            opaque_login_finish,
        } = params;

        let Client2FaAuth {
            client_id,
            opaque_finish,
        } = auth_method;

        // Look up the initial client's ClientCredential, as well as the OPAQUE
        // state in the ephemeral DB based on the client_id
        let (client_credential, opaque_state) = ephemeral_storage_provider
            .load_login_state(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?
            .ok_or(FinishUserRegistrationError::ClientCredentialNotFound)?;

        // Finish the OPAQUE handshake
        opaque_state
            .finish(opaque_login_finish.client_message)
            .map_err(|e| {
                tracing::error!("Error during OPAQUE login handshake: {e}");
                FinishUserRegistrationError::OpaqueLoginFinishFailed
            })?;

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
            .delete_login_state(&client_id)
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
