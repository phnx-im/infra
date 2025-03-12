// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::auth_service::{ClientKeyPackageError, PublishConnectionPackageError},
    messages::client_as::{
        AsClientConnectionPackageResponse, AsPublishConnectionPackagesParamsTbs,
        ClientConnectionPackageParamsTbs, ConnectionPackage,
    },
};

use crate::auth_service::{
    connection_package::StorableConnectionPackage,
    credentials::intermediate_signing_key::IntermediateCredential, AuthService,
};

impl AuthService {
    pub(crate) async fn as_publish_connection_packages(
        &self,
        params: AsPublishConnectionPackagesParamsTbs,
    ) -> Result<(), PublishConnectionPackageError> {
        let AsPublishConnectionPackagesParamsTbs {
            client_id,
            connection_packages,
        } = params;

        let as_intermediate_credentials = IntermediateCredential::load_all(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Error loading intermediate credentials: {:?}", e);
                PublishConnectionPackageError::StorageError
            })?;

        // TODO: Last resort connection package
        let connection_packages = connection_packages
            .into_iter()
            .map(|cp| {
                let verifying_credential = as_intermediate_credentials
                    .iter()
                    .find(|aic| aic.fingerprint() == cp.client_credential_signer_fingerprint())
                    .ok_or(PublishConnectionPackageError::InvalidKeyPackage)?;
                cp.verify(verifying_credential.verifying_key())
                    .map_err(|_| PublishConnectionPackageError::InvalidKeyPackage)
            })
            .collect::<Result<Vec<ConnectionPackage>, PublishConnectionPackageError>>()?;

        StorableConnectionPackage::store_multiple(&self.db_pool, &connection_packages, &client_id)
            .await
            .map_err(|_| PublishConnectionPackageError::StorageError)?;
        Ok(())
    }

    pub(crate) async fn as_client_key_package(
        &self,
        params: ClientConnectionPackageParamsTbs,
    ) -> Result<AsClientConnectionPackageResponse, ClientKeyPackageError> {
        let client_id = params.0;

        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::error!("Can't acquire a connection: {:?}", e);
            ClientKeyPackageError::StorageError
        })?;
        let connection_package =
            StorableConnectionPackage::client_connection_package(&mut connection, &client_id)
                .await
                .map_err(|e| {
                    tracing::error!("Storage provider error: {:?}", e);
                    ClientKeyPackageError::StorageError
                })?;

        let response = AsClientConnectionPackageResponse {
            connection_package: Some(connection_package),
        };
        Ok(response)
    }
}
