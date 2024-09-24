// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{OpenMlsProvider, ProtocolVersion},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use phnxtypes::{
    crypto::{
        ear::{EarDecryptable, EarEncryptable},
        signatures::signable::Signable,
    },
    errors::qs::{
        QsClientKeyPackageError, QsEncryptionKeyError, QsKeyPackageBatchError,
        QsPublishKeyPackagesError, QsVerifyingKeyError,
    },
    keypackage_batch::{AddPackage, AddPackageIn, KeyPackageBatchTbs},
    messages::client_qs::{
        ClientKeyPackageParams, ClientKeyPackageResponse, EncryptionKeyResponse,
        KeyPackageBatchParams, KeyPackageBatchResponse, PublishKeyPackagesParams,
        VerifyingKeyResponse,
    },
    time::TimeStamp,
};

use crate::qs::{storage_provider_trait::QsStorageProvider, Qs};

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

        let mut verified_add_packages = vec![];
        let mut last_resort_add_package = None;
        for add_package in add_packages {
            let verified_add_package: AddPackage = add_package
                .validate(
                    OpenMlsRustCrypto::default().crypto(),
                    ProtocolVersion::default(),
                )
                .map_err(|_| QsPublishKeyPackagesError::InvalidKeyPackage)?;
            if verified_add_package.key_package().last_resort() {
                // For now, we only allow the upload of one last resort add
                // package at a time and ignore all following add packages.
                last_resort_add_package = Some(
                    verified_add_package
                        .encrypt(&friendship_ear_key)
                        .map_err(|_| QsPublishKeyPackagesError::LibraryError)?,
                );
            } else {
                verified_add_packages.push(verified_add_package);
            }
        }

        let encrypted_add_packages = verified_add_packages
            .into_iter()
            .map(|ap| {
                ap.encrypt(&friendship_ear_key)
                    .map_err(|_| QsPublishKeyPackagesError::LibraryError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(last_resort_add_package) = last_resort_add_package {
            storage_provider
                .store_last_resort_key_package(&sender, last_resort_add_package)
                .await
                .map_err(|_| QsPublishKeyPackagesError::StorageError)?;
        }
        storage_provider
            .store_key_packages(&sender, encrypted_add_packages)
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
        &self,
        storage_provider: &S,
        params: KeyPackageBatchParams,
    ) -> Result<KeyPackageBatchResponse, QsKeyPackageBatchError> {
        let KeyPackageBatchParams {
            sender,
            friendship_ear_key,
        } = params;

        let encrypted_key_packages = storage_provider
            .load_user_key_packages(&sender)
            .await
            .map_err(|e| {
                tracing::warn!("Storage provider error: {:?}", e);
                QsKeyPackageBatchError::StorageError
            })?;

        let add_packages = encrypted_key_packages
            .into_iter()
            .map(|encrypted_key_package| {
                AddPackageIn::decrypt(&friendship_ear_key, &encrypted_key_package)
                    .map_err(|_| QsKeyPackageBatchError::DecryptionError)
                    .and_then(|ap| {
                        ap.validate(
                            OpenMlsRustCrypto::default().crypto(),
                            ProtocolVersion::default(),
                        )
                        .map_err(|_| QsKeyPackageBatchError::InvalidKeyPackage)
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let key_package_refs = add_packages
            .iter()
            .map(|add_package| {
                add_package
                    .key_package()
                    .hash_ref(OpenMlsRustCrypto::default().crypto())
                    .map_err(|_| QsKeyPackageBatchError::LibraryError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let key_package_batch_tbs =
            KeyPackageBatchTbs::new(self.domain.clone(), key_package_refs, TimeStamp::now());

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

    /// Retrieve the verifying key of this QS
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_verifying_key<S: QsStorageProvider>(
        storage_provider: &S,
    ) -> Result<VerifyingKeyResponse, QsVerifyingKeyError> {
        storage_provider
            .load_signing_key()
            .await
            .map(|signing_key| {
                let verifying_key = signing_key.verifying_key().clone();
                VerifyingKeyResponse { verifying_key }
            })
            .map_err(|_| QsVerifyingKeyError::StorageError)
    }

    /// Retrieve the client id encryption key of this QS
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_encryption_key<S: QsStorageProvider>(
        storage_provider: &S,
    ) -> Result<EncryptionKeyResponse, QsEncryptionKeyError> {
        storage_provider
            .load_decryption_key()
            .await
            .map(|decryption_key| {
                let encryption_key = decryption_key.encryption_key().clone();
                EncryptionKeyResponse { encryption_key }
            })
            .map_err(|_| QsEncryptionKeyError::StorageError)
    }
}
