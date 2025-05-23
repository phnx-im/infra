// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    identifiers::{UserHandleHash, UserId},
    messages::{client_as::ConnectionPackage, client_as_out::ConnectionPackageIn},
};
use tracing::error;

use crate::{
    auth_service::{
        AuthService, connection_package::StorableConnectionPackage,
        credentials::intermediate_signing_key::IntermediateCredential,
    },
    errors::auth_service::PublishConnectionPackageError,
};

impl AuthService {
    pub(crate) async fn as_publish_connection_packages(
        &self,
        user_id: UserId,
        connection_packages: Vec<ConnectionPackageIn>,
    ) -> Result<(), PublishConnectionPackageError> {
        let as_intermediate_credentials = IntermediateCredential::load_all(&self.db_pool)
            .await
            .map_err(|error| {
                error!(%error, "Error loading intermediate credentials");
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

        StorableConnectionPackage::store_multiple(&self.db_pool, &connection_packages, &user_id)
            .await
            .map_err(|_| PublishConnectionPackageError::StorageError)?;
        Ok(())
    }

    pub(crate) async fn as_publish_connection_packages_for_handle(
        &self,
        hash: &UserHandleHash,
        connection_packages: Vec<ConnectionPackageIn>,
    ) -> Result<(), PublishConnectionPackageError> {
        let as_intermediate_credentials = IntermediateCredential::load_all(&self.db_pool)
            .await
            .map_err(|error| {
                error!(%error, "Error loading intermediate credentials");
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

        StorableConnectionPackage::store_multiple_for_handle(
            &self.db_pool,
            &connection_packages,
            hash,
        )
        .await
        .map_err(|_| PublishConnectionPackageError::StorageError)?;
        Ok(())
    }
}
