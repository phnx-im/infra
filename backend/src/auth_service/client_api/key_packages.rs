// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::{
        prelude::{KeyPackage, OpenMlsCryptoProvider},
        versions::ProtocolVersion,
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
};

use crate::{
    auth_service::{errors::*, storage_provider_trait::AsStorageProvider, AuthService},
    messages::client_as::*,
};

impl AuthService {
    pub(crate) async fn as_publish_key_packages<S: AsStorageProvider>(
        storage_provider: &S,
        params: AsPublishKeyPackagesParamsTbs,
    ) -> Result<(), PublishKeyPackageError> {
        let AsPublishKeyPackagesParamsTbs {
            client_id,
            key_packages,
        } = params;

        // TODO: Validate the key packages

        // TODO: Last resort key package
        let key_packages = key_packages
            .into_iter()
            .map(|kp| {
                kp.validate(
                    OpenMlsRustCrypto::default().crypto(),
                    ProtocolVersion::default(),
                )
                .map_err(|_| PublishKeyPackageError::InvalidKeyPackage)
            })
            .collect::<Result<Vec<KeyPackage>, PublishKeyPackageError>>()?;

        storage_provider
            .store_key_packages(&client_id, key_packages)
            .await
            .map_err(|_| PublishKeyPackageError::StorageError)?;
        Ok(())
    }

    pub(crate) async fn as_client_key_package<S: AsStorageProvider>(
        storage_provider: &S,
        params: ClientKeyPackageParamsTbs,
    ) -> Result<AsClientKeyPackageResponse, ClientKeyPackageError> {
        let client_id = params.0;

        let key_package = storage_provider
            .client_key_package(&client_id)
            .await
            .map_err(|_| ClientKeyPackageError::StorageError)?;

        let response = AsClientKeyPackageResponse { key_package };
        Ok(response)
    }
}
