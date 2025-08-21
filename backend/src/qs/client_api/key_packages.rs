// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::messages::client_qs::{
    EncryptionKeyResponse, KeyPackageParams, KeyPackageResponse, PublishKeyPackagesParams,
};
use mls_assist::{
    openmls::prelude::{KeyPackage, OpenMlsProvider, ProtocolVersion},
    openmls_rust_crypto::OpenMlsRustCrypto,
};

use crate::{
    errors::qs::{QsEncryptionKeyError, QsKeyPackageError, QsPublishKeyPackagesError},
    qs::{
        Qs, client_id_decryption_key::StorableClientIdDecryptionKey,
        key_package::StorableKeyPackage,
    },
};

impl Qs {
    /// Clients publish key packages to the server.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_publish_key_packages(
        &self,
        params: PublishKeyPackagesParams,
    ) -> Result<(), QsPublishKeyPackagesError> {
        let PublishKeyPackagesParams {
            sender,
            key_packages,
        } = params;

        let mut verified_key_packages = vec![];
        let mut last_resort_key_package = None;
        for key_package in key_packages {
            let verified_key_package: KeyPackage = key_package
                .validate(
                    OpenMlsRustCrypto::default().crypto(),
                    ProtocolVersion::default(),
                )
                .map_err(|_| QsPublishKeyPackagesError::InvalidKeyPackage)?;

            let is_last_resort = verified_key_package.last_resort();

            if is_last_resort {
                last_resort_key_package = Some(verified_key_package);
            } else {
                verified_key_packages.push(verified_key_package);
            }
        }

        if let Some(last_resort_key_package) = last_resort_key_package {
            last_resort_key_package
                .store_last_resort(&self.db_pool, &sender)
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to store last resort key package: {:?}", e);
                    QsPublishKeyPackagesError::StorageError
                })?;
        }

        KeyPackage::store_multiple(&self.db_pool, &sender, &verified_key_packages)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to store last resort key package: {:?}", e);
                QsPublishKeyPackagesError::StorageError
            })?;

        Ok(())
    }

    /// Retrieve a key package for a given client.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_key_package(
        &self,
        params: KeyPackageParams,
    ) -> Result<KeyPackageResponse, QsKeyPackageError> {
        let KeyPackageParams { sender } = params;

        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection: {:?}", e);
            QsKeyPackageError::StorageError
        })?;

        let key_package = KeyPackage::load_user_key_package(&mut connection, &sender)
            .await
            .map_err(|e| {
                tracing::warn!("Storage provider error: {:?}", e);
                QsKeyPackageError::StorageError
            })?;

        let response = KeyPackageResponse { key_package };
        Ok(response)
    }

    /// Retrieve the client id encryption key of this QS
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_encryption_key(
        &self,
    ) -> Result<EncryptionKeyResponse, QsEncryptionKeyError> {
        StorableClientIdDecryptionKey::load(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to load client id decryption key: {:?}", e);
                QsEncryptionKeyError::StorageError
            })?
            .map(|decryption_key| {
                let encryption_key = decryption_key.encryption_key().clone();
                EncryptionKeyResponse { encryption_key }
            })
            .ok_or(QsEncryptionKeyError::LibraryError)
    }
}
