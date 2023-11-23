// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::{rand::rngs::OsRng, ServerLogin, ServerLoginStartParameters};
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{opaque::OpaqueLoginResponse, signatures::signable::Signable, OpaqueCiphersuite},
    errors::auth_service::{
        AsDequeueError, DeleteClientError, FinishClientAdditionError, InitClientAdditionError,
    },
    messages::{
        client_as::{
            DeleteClientParamsTbs, DequeueMessagesParamsTbs, FinishClientAdditionParamsTbs,
            InitClientAdditionResponse, InitiateClientAdditionParams,
        },
        client_qs::DequeueMessagesResponse,
    },
    time::TimeStamp,
};
use privacypass::Serialize;

use crate::auth_service::{
    storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
    AsClientRecord, AuthService,
};

impl AuthService {
    pub(crate) async fn as_init_client_addition<
        S: AsStorageProvider,
        E: AsEphemeralStorageProvider,
    >(
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: InitiateClientAdditionParams,
    ) -> Result<InitClientAdditionResponse, InitClientAdditionError> {
        let InitiateClientAdditionParams {
            client_credential_payload,
            opaque_login_request,
        } = params;

        // Load the server setup from storage
        let server_setup = storage_provider.load_opaque_setup().await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            InitClientAdditionError::StorageError
        })?;

        // Load the user record from storage
        let user_name = client_credential_payload.identity().user_name();
        let password_file_option = storage_provider
            .load_user(&user_name)
            .await
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
            .store_client_login_state(
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

    pub(crate) async fn as_finish_client_addition<
        S: AsStorageProvider,
        E: AsEphemeralStorageProvider,
    >(
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: FinishClientAdditionParamsTbs,
    ) -> Result<(), FinishClientAdditionError> {
        let FinishClientAdditionParamsTbs {
            client_id,
            queue_encryption_key,
            initial_ratchet_secret: initial_ratchet_key,
            connection_package: connection_key_package,
        } = params;

        // Look up the initial client's ClientCredentialn the ephemeral DB based
        // on the client_id
        let (client_credential, _opaque_state) = ephemeral_storage_provider
            .load_client_login_state(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishClientAdditionError::StorageError
            })?
            .ok_or(FinishClientAdditionError::ClientCredentialNotFound)?;

        // Create the new client entry
        let client_record = AsClientRecord {
            queue_encryption_key,
            ratchet_key: initial_ratchet_key
                .try_into()
                // Hiding the LibraryError here behind a StorageError
                .map_err(|_| FinishClientAdditionError::StorageError)?,
            activity_time: TimeStamp::now(),
            credential: client_credential,
        };

        storage_provider
            .store_client(&client_id, &client_record)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishClientAdditionError::StorageError
            })?;

        // Delete the entry in the ephemeral OPAQUE DB
        ephemeral_storage_provider
            .delete_client_login_state(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishClientAdditionError::StorageError
            })?;

        Ok(())
    }

    pub(crate) async fn as_delete_client<S: AsStorageProvider>(
        storage_provider: &S,
        params: DeleteClientParamsTbs,
    ) -> Result<(), DeleteClientError> {
        let client_id = params.0;

        // Delete the client
        storage_provider
            .delete_client(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                DeleteClientError::StorageError
            })?;

        Ok(())
    }

    pub(crate) async fn as_dequeue_messages<S: AsStorageProvider>(
        storage_provider: &S,
        params: DequeueMessagesParamsTbs,
    ) -> Result<DequeueMessagesResponse, AsDequeueError> {
        let DequeueMessagesParamsTbs {
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
