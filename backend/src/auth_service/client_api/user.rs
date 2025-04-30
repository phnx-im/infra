// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    credentials::ClientCredential,
    crypto::signatures::signable::Signable,
    errors::auth_service::{DeleteUserError, InitUserRegistrationError},
    messages::{
        client_as::{DeleteUserParamsTbs, InitUserRegistrationResponse},
        client_as_out::InitUserRegistrationParamsIn,
    },
    time::TimeStamp,
};
use tracing::error;

use crate::auth_service::{
    AuthService, client_record::ClientRecord,
    credentials::intermediate_signing_key::IntermediateSigningKey, user_record::UserRecord,
};

impl AuthService {
    pub(crate) async fn as_init_user_registration(
        &self,
        params: InitUserRegistrationParamsIn,
    ) -> Result<InitUserRegistrationResponse, InitUserRegistrationError> {
        let InitUserRegistrationParamsIn {
            client_payload,
            queue_encryption_key,
            initial_ratchet_secret,
            encrypted_user_profile,
        } = params;

        // Check if a user entry with the name given in the client_csr already exists
        tracing::info!("Checking if user already exists");
        let user_name_exists =
            UserRecord::load(&self.db_pool, client_payload.identity().user_name())
                .await
                .map_err(|error| {
                    error!(%error, "Error loading user record");
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
            .map_err(|error| {
                error!(%error, "Error loading signing key");
                InitUserRegistrationError::StorageError
            })?
            .ok_or(InitUserRegistrationError::SigningKeyNotFound)?;

        // Sign the credential
        let client_credential: ClientCredential = client_payload
            .sign(&signing_key)
            .map_err(|_| InitUserRegistrationError::LibraryError)?;

        let client_id = client_credential.identity();

        // Create the user entry with the information given in the request
        UserRecord::new_and_store(
            &self.db_pool,
            client_id.user_name(),
            &encrypted_user_profile,
        )
        .await
        .map_err(|error| {
            error!(%error, "Storage provider error");
            InitUserRegistrationError::StorageError
        })?;

        // Create the initial client entry
        let ratchet_key = initial_ratchet_secret
            .try_into()
            // Hiding the LibraryError here behind a StorageError
            .map_err(|_| InitUserRegistrationError::StorageError)?;
        let mut connection = self.db_pool.acquire().await.map_err(|error| {
            error!(%error, "Error acquiring connection");
            InitUserRegistrationError::StorageError
        })?;
        ClientRecord::new_and_store(
            &mut connection,
            queue_encryption_key,
            ratchet_key,
            client_credential.clone(),
        )
        .await
        .map_err(|error| {
            error!(%error, "Storage provider error");
            InitUserRegistrationError::StorageError
        })?;

        let response = InitUserRegistrationResponse { client_credential };

        Ok(response)
    }

    pub(crate) async fn as_delete_user(
        &self,
        params: DeleteUserParamsTbs,
    ) -> Result<(), DeleteUserError> {
        let DeleteUserParamsTbs {
            user_name,
            client_id: _,
        } = params;

        // Delete the user
        UserRecord::delete(&self.db_pool, &user_name)
            .await
            .map_err(|error| {
                error!(%error, "Storage provider error");
                DeleteUserError::StorageError
            })?;

        Ok(())
    }
}
