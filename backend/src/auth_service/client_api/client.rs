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
            ConnectionPackage, DeleteClientParamsTbs, DequeueMessagesParamsTbs,
            FinishClientAdditionParamsTbs, InitClientAdditionResponse,
            InitiateClientAdditionParams,
        },
        client_qs::DequeueMessagesResponse,
    },
    time::TimeStamp,
};
use tls_codec::Serialize;

use crate::auth_service::{
    client_record::ClientRecord,
    connection_package::StorableConnectionPackage,
    credentials::intermediate_signing_key::{IntermediateCredential, IntermediateSigningKey},
    opaque::OpaqueSetup,
    queue::Queue,
    user_record::UserRecord,
    AuthService,
};

impl AuthService {
    pub(crate) async fn as_init_client_addition(
        &self,
        params: InitiateClientAdditionParams,
    ) -> Result<InitClientAdditionResponse, InitClientAdditionError> {
        let InitiateClientAdditionParams {
            client_credential_payload,
            opaque_login_request,
        } = params;

        // Load the server setup from storage
        let server_setup = OpaqueSetup::load(&self.db_pool).await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            InitClientAdditionError::StorageError
        })?;

        // Load the user record from storage
        let user_name = client_credential_payload.identity().user_name();
        let password_file_option = UserRecord::load(&self.db_pool, &user_name)
            .await
            .map_err(|e| {
                tracing::error!("Error loading user record: {:?}", e);
                InitClientAdditionError::StorageError
            })?
            .map(|record| record.into_password_file());

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
        let client_id_exists =
            ClientRecord::load(&self.db_pool, &client_credential_payload.identity())
                .await
                .map_err(|e| {
                    tracing::error!("Error loading client record: {:?}", e);
                    InitClientAdditionError::StorageError
                })?
                .is_some();

        if client_id_exists {
            return Err(InitClientAdditionError::ClientAlreadyExists);
        }

        // Validate the client credential payload
        if !client_credential_payload.validate() {
            let now = TimeStamp::now();
            let not_before = client_credential_payload.expiration_data().not_before();
            let not_after = client_credential_payload.expiration_data().not_after();
            return Err(InitClientAdditionError::InvalidCsr(
                now, not_before, not_after,
            ));
        }

        // Load the signature key from storage.
        let signing_key = IntermediateSigningKey::load(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Error loading signing key: {:?}", e);
                InitClientAdditionError::StorageError
            })?
            .ok_or(InitClientAdditionError::SigningKeyNotFound)?;

        // Sign the credential
        let client_credential: ClientCredential = client_credential_payload
            .sign(&signing_key)
            .map_err(|_| InitClientAdditionError::LibraryError)?;

        // Store the client_credential in the ephemeral DB
        let mut client_credentials = self.ephemeral_client_credentials.lock().await;
        client_credentials.insert(
            client_credential.identity().clone(),
            client_credential.clone(),
        );

        let response = InitClientAdditionResponse {
            client_credential,
            opaque_login_response,
        };

        Ok(response)
    }

    pub(crate) async fn as_finish_client_addition(
        &self,
        params: FinishClientAdditionParamsTbs,
    ) -> Result<(), FinishClientAdditionError> {
        let FinishClientAdditionParamsTbs {
            client_id,
            queue_encryption_key,
            initial_ratchet_secret: initial_ratchet_key,
            connection_package,
        } = params;

        // Look up the initial client's ClientCredentialn the ephemeral DB based
        // on the client_id
        let mut client_credentials = self.ephemeral_client_credentials.lock().await;
        let client_credential = client_credentials
            .remove(&client_id)
            .ok_or(FinishClientAdditionError::ClientCredentialNotFound)?;

        // Create the new client entry
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::error!("Error acquiring connection: {:?}", e);
            FinishClientAdditionError::StorageError
        })?;
        let ratchet_key = initial_ratchet_key
            .try_into()
            // Hiding the LibraryError here behind a StorageError
            .map_err(|_| FinishClientAdditionError::StorageError)?;
        ClientRecord::new_and_store(
            &mut connection,
            queue_encryption_key,
            ratchet_key,
            client_credential,
        )
        .await
        .map_err(|e| {
            tracing::error!("Error storing client record: {:?}", e);
            FinishClientAdditionError::StorageError
        })?;

        // Verify and store connection packages
        let as_intermediate_credentials = IntermediateCredential::load_all(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Error loading intermediate credentials: {:?}", e);
                FinishClientAdditionError::StorageError
            })?;
        let cp = connection_package;
        let verifying_credential = as_intermediate_credentials
            .iter()
            .find(|aic| aic.fingerprint() == cp.client_credential_signer_fingerprint())
            .ok_or(FinishClientAdditionError::InvalidConnectionPackage)?;
        let verified_connection_package: ConnectionPackage = cp
            .verify(verifying_credential.verifying_key())
            .map_err(|_| FinishClientAdditionError::InvalidConnectionPackage)?;

        StorableConnectionPackage::store_multiple(
            &self.db_pool,
            [&verified_connection_package],
            &client_id,
        )
        .await
        .map_err(|e| {
            tracing::error!("Error storing connection package: {:?}", e);
            FinishClientAdditionError::StorageError
        })?;

        // Delete the entry in the ephemeral OPAQUE DB
        let mut client_login_states = self.ephemeral_client_logins.lock().await;
        client_login_states.remove(&client_id);

        Ok(())
    }

    pub(crate) async fn as_delete_client(
        &self,
        params: DeleteClientParamsTbs,
    ) -> Result<(), DeleteClientError> {
        let client_id = params.0;

        // Delete the client
        ClientRecord::delete(&self.db_pool, &client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                DeleteClientError::StorageError
            })?;

        Ok(())
    }

    pub(crate) async fn as_dequeue_messages(
        &self,
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
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::error!("Error acquiring connection: {:?}", e);
            AsDequeueError::StorageError
        })?;
        let (messages, remaining_messages_number) = Queue::read_and_delete(
            &mut connection,
            &sender,
            sequence_number_start,
            max_message_number,
        )
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
