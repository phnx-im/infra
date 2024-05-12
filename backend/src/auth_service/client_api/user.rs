// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::ServerRegistration;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{signatures::signable::Signable, OpaqueCiphersuite},
    errors::auth_service::{
        DeleteUserError, FinishUserRegistrationError, InitUserRegistrationError,
    },
    messages::{
        client_as::{
            DeleteUserParamsTbs, InitUserRegistrationParams, InitUserRegistrationResponse,
        },
        client_as_out::FinishUserRegistrationParamsTbsIn,
    },
    time::TimeStamp,
};
use tls_codec::Serialize;

use crate::auth_service::{
    AsClientRecord, AsEphemeralStorageProvider, AsStorageProvider, AuthService,
};

impl AuthService {
    pub(crate) async fn as_init_user_registration<
        S: AsStorageProvider,
        E: AsEphemeralStorageProvider,
    >(
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: InitUserRegistrationParams,
    ) -> Result<InitUserRegistrationResponse, InitUserRegistrationError> {
        let InitUserRegistrationParams {
            client_payload,
            opaque_registration_request,
        } = params;

        // Check if a user entry with the name given in the client_csr already exists
        let client_id_exists = storage_provider
            .load_user(&client_payload.identity().user_name())
            .await
            .is_some();

        if client_id_exists {
            return Err(InitUserRegistrationError::UserAlreadyExists);
        }

        // Validate the client_csr
        if !client_payload.validate() {
            let now = TimeStamp::now();
            let not_before = client_payload.expiration_data().not_before();
            let not_after = client_payload.expiration_data().not_after();
            return Err(InitUserRegistrationError::InvalidCsr(
                now, not_before, not_after,
            ));
        }

        // Load the signature key from storage.
        let signing_key = storage_provider.load_signing_key().await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            InitUserRegistrationError::StorageError
        })?;

        // Sign the credential
        let client_credential: ClientCredential = client_payload
            .sign(&signing_key)
            .map_err(|_| InitUserRegistrationError::LibraryError)?;

        // Store the client_credential in the ephemeral DB
        ephemeral_storage_provider
            .store_credential(client_credential.identity(), &client_credential)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                InitUserRegistrationError::StorageError
            })?;

        // Perform OPAQUE registration

        // Load server key material
        let server_setup = storage_provider.load_opaque_setup().await.map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            InitUserRegistrationError::StorageError
        })?;

        // Perform server operation
        let registration_result = ServerRegistration::<OpaqueCiphersuite>::start(
            &server_setup,
            opaque_registration_request.client_message,
            &client_credential
                .identity()
                .user_name()
                .tls_serialize_detached()
                .map_err(|_| InitUserRegistrationError::LibraryError)?,
        )
        .map_err(|_| InitUserRegistrationError::OpaqueRegistrationFailed)?;

        let opaque_registration_response = registration_result.message.into();

        let response = InitUserRegistrationResponse {
            client_credential,
            opaque_registration_response,
        };

        Ok(response)
    }

    pub(crate) async fn as_finish_user_registration<
        S: AsStorageProvider,
        E: AsEphemeralStorageProvider,
    >(
        storage_provider: &S,
        ephemeral_storage_provider: &E,
        params: FinishUserRegistrationParamsTbsIn,
    ) -> Result<(), FinishUserRegistrationError> {
        let FinishUserRegistrationParamsTbsIn {
            client_id,
            queue_encryption_key,
            initial_ratchet_secret: initial_ratchet_key,
            connection_packages,
            opaque_registration_record,
        } = params;

        // Look up the initial client's ClientCredential in the ephemeral DB based on the user_name
        let client_credential = ephemeral_storage_provider
            .load_credential(&client_id)
            .await
            .ok_or(FinishUserRegistrationError::ClientCredentialNotFound)?;

        // Authenticate the request using the signature key in the
        // ClientCredential

        // TODO: This is tricky, since we cannot do this ahead
        // of time, since the client certificate is only in the ephemeral DB.

        // Finish OPAQUE flow
        let password_file = ServerRegistration::finish(opaque_registration_record.client_message);

        // Create the user entry with the information given in the request
        storage_provider
            .create_user(&client_id.user_name(), &password_file)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        // Verify and store connection packages
        let (_as_credentials, as_intermediate_credentials, _revoked_fingerprints) =
            storage_provider.load_as_credentials().await.map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;
        let verified_connection_packages = connection_packages
            .into_iter()
            .map(|cp| {
                let verifying_credential = as_intermediate_credentials
                    .iter()
                    .find(|aic| aic.fingerprint() == cp.client_credential_signer_fingerprint())
                    .ok_or(FinishUserRegistrationError::InvalidConnectionPackage)?;
                cp.verify(verifying_credential.verifying_key())
                    .map_err(|_| FinishUserRegistrationError::InvalidConnectionPackage)
            })
            .collect::<Result<Vec<_>, FinishUserRegistrationError>>()?;

        storage_provider
            .store_connection_packages(&client_id, verified_connection_packages)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        // Create the initial client entry

        let client_record = AsClientRecord {
            queue_encryption_key,
            ratchet_key: initial_ratchet_key
                .try_into()
                // Hiding the LibraryError here behind a StorageError
                .map_err(|_| FinishUserRegistrationError::StorageError)?,
            activity_time: TimeStamp::now(),
            credential: client_credential,
        };

        storage_provider
            .create_client(&client_id, &client_record)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        // Delete the entry in the ephemeral OPAQUE DB
        ephemeral_storage_provider
            .delete_client_login_state(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;
        ephemeral_storage_provider
            .delete_credential(&client_id)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        Ok(())
    }

    pub(crate) async fn as_delete_user<S: AsStorageProvider>(
        storage_provider: &S,
        params: DeleteUserParamsTbs,
    ) -> Result<(), DeleteUserError> {
        let DeleteUserParamsTbs {
            user_name,
            client_id,
            opaque_finish: _,
        } = params;

        // Delete the user
        storage_provider
            .delete_user(&client_id.user_name())
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                DeleteUserError::StorageError
            })?;

        Ok(())
    }
}
