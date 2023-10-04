// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnx_types::messages::client_as::{
    AsClientConnectionPackageResponse, AsPublishConnectionPackagesParamsTbs,
    ClientConnectionPackageParamsTbs, ConnectionPackage,
};

use crate::auth_service::{errors::*, storage_provider_trait::AsStorageProvider, AuthService};

impl AuthService {
    pub(crate) async fn as_publish_connection_packages<S: AsStorageProvider>(
        storage_provider: &S,
        params: AsPublishConnectionPackagesParamsTbs,
    ) -> Result<(), PublishConnectionPackageError> {
        let AsPublishConnectionPackagesParamsTbs {
            client_id,
            connection_packages,
        } = params;

        let (_, as_intermediate_credentials, _) = storage_provider
            .load_as_credentials()
            .await
            .map_err(|_| PublishConnectionPackageError::StorageError)?;

        // TODO: Last resort key package
        let connection_packages = connection_packages
            .into_iter()
            .map(|cp| {
                let verifying_credential = as_intermediate_credentials
                    .iter()
                    .find(|aic| {
                        if let Ok(fingerprint) = aic.fingerprint() {
                            &fingerprint == cp.client_credential_signer_fingerprint()
                        } else {
                            false
                        }
                    })
                    .ok_or(PublishConnectionPackageError::InvalidKeyPackage)?;
                cp.verify(verifying_credential.verifying_key())
                    .map_err(|_| PublishConnectionPackageError::InvalidKeyPackage)
            })
            .collect::<Result<Vec<ConnectionPackage>, PublishConnectionPackageError>>()?;

        storage_provider
            .store_connection_packages(&client_id, connection_packages)
            .await
            .map_err(|_| PublishConnectionPackageError::StorageError)?;
        Ok(())
    }

    pub(crate) async fn as_client_key_package<S: AsStorageProvider>(
        storage_provider: &S,
        params: ClientConnectionPackageParamsTbs,
    ) -> Result<AsClientConnectionPackageResponse, ClientKeyPackageError> {
        let client_id = params.0;

        let connection_package = storage_provider.client_connection_package(&client_id).await;

        let response = AsClientConnectionPackageResponse { connection_package };
        Ok(response)
    }
}
