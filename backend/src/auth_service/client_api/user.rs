// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::ServerRegistration;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{OpaqueCiphersuite, signatures::signable::Signable},
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
    AuthService,
    client_record::ClientRecord,
    connection_package::StorableConnectionPackage,
    credentials::intermediate_signing_key::{IntermediateCredential, IntermediateSigningKey},
    opaque::OpaqueSetup,
    user_record::UserRecord,
};

impl AuthService {
    pub(crate) async fn as_init_user_registration(
        &self,
        params: InitUserRegistrationParams,
    ) -> Result<InitUserRegistrationResponse, InitUserRegistrationError> {
        let InitUserRegistrationParams {
            client_payload,
            opaque_registration_request,
        } = params;

        // Check if a user entry with the name given in the client_csr already exists
        tracing::info!("Checking if user already exists");
        let user_name_exists =
            UserRecord::load(&self.db_pool, &client_payload.identity().user_name())
                .await
                .map_err(|e| {
                    tracing::error!("Error loading user record: {:?}", e);
                    InitUserRegistrationError::StorageError
                })?
                .is_some();

        if user_name_exists {
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
        let signing_key = IntermediateSigningKey::load(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Error loading signing key: {:?}", e);
                InitUserRegistrationError::StorageError
            })?
            .ok_or(InitUserRegistrationError::SigningKeyNotFound)?;

        // Sign the credential
        let client_credential: ClientCredential = client_payload
            .sign(&signing_key)
            .map_err(|_| InitUserRegistrationError::LibraryError)?;

        // Store the client_credential in the ephemeral DB
        let mut client_credentials = self.ephemeral_client_credentials.lock().await;
        client_credentials.insert(
            client_credential.identity().clone(),
            client_credential.clone(),
        );

        // Perform OPAQUE registration

        // Load server key material
        let server_setup = OpaqueSetup::load(&self.db_pool).await.map_err(|e| {
            tracing::error!("Error loading OPAQUE setup: {:?}", e);
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

    pub(crate) async fn as_finish_user_registration(
        &self,
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
        let mut client_credentials = self.ephemeral_client_credentials.lock().await;
        let client_credential = client_credentials
            .remove(&client_id)
            .ok_or(FinishUserRegistrationError::ClientCredentialNotFound)?;

        // Authenticate the request using the signature key in the
        // ClientCredential

        // Finish OPAQUE flow
        let password_file = ServerRegistration::finish(opaque_registration_record.client_message);

        // Create the user entry with the information given in the request
        UserRecord::new_and_store(&self.db_pool, &client_id.user_name(), &password_file)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                FinishUserRegistrationError::StorageError
            })?;

        // Verify and store connection packages
        let as_intermediate_credentials = IntermediateCredential::load_all(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Error loading intermediate credentials: {:?}", e);
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

        // Create the initial client entry
        let ratchet_key = initial_ratchet_key
            .try_into()
            // Hiding the LibraryError here behind a StorageError
            .map_err(|_| FinishUserRegistrationError::StorageError)?;
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::error!("Error acquiring connection: {:?}", e);
            FinishUserRegistrationError::StorageError
        })?;
        ClientRecord::new_and_store(
            &mut connection,
            queue_encryption_key,
            ratchet_key,
            client_credential,
        )
        .await
        .map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            FinishUserRegistrationError::StorageError
        })?;

        StorableConnectionPackage::store_multiple(
            &self.db_pool,
            &verified_connection_packages,
            &client_id,
        )
        .await
        .map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            FinishUserRegistrationError::StorageError
        })?;

        // Delete the entry in the ephemeral OPAQUE DB
        let mut client_login_states = self.ephemeral_client_logins.lock().await;
        client_login_states.remove(&client_id);
        Ok(())
    }

    pub(crate) async fn as_delete_user(
        &self,
        params: DeleteUserParamsTbs,
    ) -> Result<(), DeleteUserError> {
        let DeleteUserParamsTbs {
            user_name,
            client_id: _,
            opaque_finish: _,
        } = params;

        // Delete the user
        UserRecord::delete(&self.db_pool, &user_name)
            .await
            .map_err(|e| {
                tracing::error!("Storage provider error: {:?}", e);
                DeleteUserError::StorageError
            })?;

        Ok(())
    }
}
