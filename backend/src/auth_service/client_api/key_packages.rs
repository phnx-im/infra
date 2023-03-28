// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    auth_service::{errors::*, storage_provider_trait::AsStorageProvider, AuthService},
    messages::client_as::*,
};

impl AuthService {
    pub async fn as_publish_key_packages<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: PublishKeyPackagesParams,
    ) -> Result<(), PublishKeyPackageError> {
        let PublishKeyPackagesParams {
            auth_method,
            key_packages,
        } = params;

        let ClientCredentialAuth {
            client_id,
            signature,
        } = auth_method;

        // TODO: Validate the key packages

        // TODO: Last resort key package

        storage_provider
            .store_key_packages(&client_id, key_packages)
            .await
            .map_err(|_| PublishKeyPackageError::StorageError)?;
        Ok(())
    }

    pub async fn as_client_key_package<S: AsStorageProvider>(
        &self,
        storage_provider: &S,
        params: ClientKeyPackageParams,
    ) -> Result<ClientKeyPackageResponse, ClientKeyPackageError> {
        let ClientKeyPackageParams {
            auth_method,
            client_id,
        } = params;

        let key_package = storage_provider
            .client_key_package(&client_id)
            .await
            .map_err(|_| ClientKeyPackageError::StorageError)?;

        let response = ClientKeyPackageResponse { key_package };
        Ok(response)
    }
}
