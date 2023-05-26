// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{OpenMlsCryptoProvider, ProtocolVersion},
    openmls_rust_crypto::OpenMlsRustCrypto,
};

use crate::{
    crypto::{ear::EarEncryptable, signatures::signable::Signable},
    ds::group_state::TimeStamp,
    messages::client_qs::{
        ClientKeyPackageParams, ClientKeyPackageResponse, KeyPackageBatchParams,
        KeyPackageBatchResponse, PublishKeyPackagesParams,
    },
    qs::{
        errors::{QsClientKeyPackageError, QsKeyPackageBatchError, QsPublishKeyPackagesError},
        storage_provider_trait::QsStorageProvider,
        AddPackage, KeyPackageBatchTbs, Qs,
    },
};

impl Qs {
    /// Clients publish key packages to the server.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_publish_key_packages<S: QsStorageProvider>(
        storage_provider: &S,
        params: PublishKeyPackagesParams,
    ) -> Result<(), QsPublishKeyPackagesError> {
        let PublishKeyPackagesParams {
            sender,
            add_packages,
            friendship_ear_key,
        } = params;

        let encrypted_key_packages = add_packages
            .into_iter()
            .map(|add_package_in| {
                add_package_in
                    .validate(
                        OpenMlsRustCrypto::default().crypto(),
                        ProtocolVersion::default(),
                    )
                    .map_err(|_| QsPublishKeyPackagesError::InvalidKeyPackage)
                    .and_then(|ap| {
                        ap.encrypt(&friendship_ear_key)
                            .map_err(|_| QsPublishKeyPackagesError::LibraryError)
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // TODO: Last resort key package

        storage_provider
            .store_key_packages(&sender, encrypted_key_packages)
            .await
            .map_err(|_| QsPublishKeyPackagesError::StorageError)?;
        Ok(())
    }

    /// Retrieve a key package for the given client.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_client_key_package<S: QsStorageProvider>(
        storage_provider: &S,
        params: ClientKeyPackageParams,
    ) -> Result<ClientKeyPackageResponse, QsClientKeyPackageError> {
        let ClientKeyPackageParams { sender, client_id } = params;

        let encrypted_key_package = storage_provider
            .load_key_package(&sender, &client_id)
            .await
            .ok_or(QsClientKeyPackageError::StorageError)?;

        let response = ClientKeyPackageResponse {
            encrypted_key_package,
        };
        Ok(response)
    }

    /// Retrieve a key package batch for a given client.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_key_package_batch<S: QsStorageProvider>(
        storage_provider: &S,
        params: KeyPackageBatchParams,
    ) -> Result<KeyPackageBatchResponse, QsKeyPackageBatchError> {
        let KeyPackageBatchParams {
            sender,
            friendship_ear_key,
        } = params;

        let encrypted_key_packages = storage_provider.load_user_key_packages(&sender).await;

        let add_packages = encrypted_key_packages
            .into_iter()
            .map(|encrypted_key_package| {
                AddPackage::decrypt(&friendship_ear_key, &encrypted_key_package)
                    .map_err(|_| QsKeyPackageBatchError::DecryptionError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let key_package_refs = add_packages
            .iter()
            .map(|add_package| {
                add_package
                    .key_package
                    .hash_ref(OpenMlsRustCrypto::default().crypto())
                    .map_err(|_| QsKeyPackageBatchError::LibraryError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let config = storage_provider
            .load_config()
            .await
            .map_err(|_| QsKeyPackageBatchError::StorageError)?;

        let key_package_batch_tbs = KeyPackageBatchTbs {
            homeserver_domain: config.fqdn.clone(),
            key_package_refs,
            time_of_signature: TimeStamp::now(),
        };

        let signing_key = storage_provider
            .load_signing_key()
            .await
            .map_err(|_| QsKeyPackageBatchError::StorageError)?;

        let key_package_batch = key_package_batch_tbs
            .sign(&signing_key)
            .map_err(|_| QsKeyPackageBatchError::LibraryError)?;

        let response = KeyPackageBatchResponse {
            add_packages,
            key_package_batch,
        };
        Ok(response)
    }
}
