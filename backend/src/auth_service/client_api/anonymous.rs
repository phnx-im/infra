// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::auth_service::{AsCredentialsError, EnqueueMessageError, UserConnectionPackagesError},
    messages::client_as::{
        AsCredentialsParams, AsCredentialsResponse, EnqueueMessageParams,
        UserConnectionPackagesParams, UserConnectionPackagesResponse,
    },
};

use crate::auth_service::{
    AuthService,
    client_record::ClientRecord,
    connection_package::StorableConnectionPackage,
    credentials::{intermediate_signing_key::IntermediateCredential, signing_key::Credential},
    queue::Queue,
};

impl AuthService {
    pub async fn as_user_connection_packages(
        &self,
        params: UserConnectionPackagesParams,
    ) -> Result<UserConnectionPackagesResponse, UserConnectionPackagesError> {
        let UserConnectionPackagesParams { user_name } = params;

        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection from pool: {:?}", e);
            UserConnectionPackagesError::StorageError
        })?;
        let connection_packages =
            StorableConnectionPackage::user_connection_packages(&mut connection, &user_name)
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

    pub(crate) async fn as_enqueue_message(
        &self,
        params: EnqueueMessageParams,
    ) -> Result<(), EnqueueMessageError> {
        let EnqueueMessageParams {
            client_id,
            connection_establishment_ctxt,
        } = params;

        // Fetch the client record.
        let mut client_record = ClientRecord::load(&self.db_pool, &client_id)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to load client record: {:?}", e);
                EnqueueMessageError::StorageError
            })?
            .ok_or(EnqueueMessageError::ClientNotFound)?;

        let payload = connection_establishment_ctxt
            .try_into()
            .map_err(|_| EnqueueMessageError::LibraryError)?;

        let queue_message = client_record
            .ratchet
            .encrypt(payload)
            .map_err(|_| EnqueueMessageError::LibraryError)?;

        // TODO: Future work: PCS

        tracing::trace!("Enqueueing message in storage provider");
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection from pool: {:?}", e);
            EnqueueMessageError::StorageError
        })?;
        Queue::enqueue(&mut connection, &client_id, &queue_message)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to enqueue message: {:?}", e);
                EnqueueMessageError::StorageError
            })?;

        // Store the changed client record.
        client_record.update(&self.db_pool).await.map_err(|e| {
            tracing::warn!("Failed to store client record: {:?}", e);
            EnqueueMessageError::StorageError
        })?;

        Ok(())
    }

    pub(crate) async fn as_credentials(
        &self,
        _params: AsCredentialsParams,
    ) -> Result<AsCredentialsResponse, AsCredentialsError> {
        let as_credentials = Credential::load_all(&self.db_pool).await.map_err(|e| {
            tracing::error!("Error loading AS credentials: {:?}", e);
            AsCredentialsError::StorageError
        })?;
        let as_intermediate_credentials = IntermediateCredential::load_all(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Error loading intermediate credentials: {:?}", e);
                AsCredentialsError::StorageError
            })?;
        Ok(AsCredentialsResponse {
            as_credentials,
            as_intermediate_credentials,
            // We don't support revocation yet
            revoked_credentials: vec![],
        })
    }
}
