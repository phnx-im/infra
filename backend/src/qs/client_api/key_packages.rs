// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{KeyPackage, KeyPackageIn, OpenMlsProvider, ProtocolVersion},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use phnxtypes::{
    crypto::ear::{EarDecryptable, EarEncryptable},
    errors::qs::{
        QsClientKeyPackageError, QsEncryptionKeyError, QsKeyPackageError, QsPublishKeyPackagesError,
    },
    messages::client_qs::{
        ClientKeyPackageParams, ClientKeyPackageResponse, EncryptionKeyResponse, KeyPackageParams,
        KeyPackageResponse, PublishKeyPackagesParams,
    },
};

use crate::qs::{
    client_id_decryption_key::StorableClientIdDecryptionKey,
    key_package::StorableEncryptedAddPackage, Qs,
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
            friendship_ear_key,
        } = params;

        let mut encrypted_key_packages = vec![];
        let mut last_resort_key_package = None;
        for key_package in key_packages {
            let verified_key_package: KeyPackage = key_package
                .validate(
                    OpenMlsRustCrypto::default().crypto(),
                    ProtocolVersion::default(),
                )
                .map_err(|_| QsPublishKeyPackagesError::InvalidKeyPackage)?;

            let is_last_resort = verified_key_package.last_resort();

            let eap = verified_key_package
                .encrypt(&friendship_ear_key)
                .map_err(|_| QsPublishKeyPackagesError::LibraryError)?;

            if is_last_resort {
                last_resort_key_package = Some(eap);
            } else {
                encrypted_key_packages.push(eap);
            }
        }

        if let Some(last_resort_key_package) = last_resort_key_package {
            StorableEncryptedAddPackage::store_last_resort(
                &self.db_pool,
                &sender,
                &last_resort_key_package,
            )
            .await
            .map_err(|e| {
                tracing::warn!("Failed to store last resort key package: {:?}", e);
                QsPublishKeyPackagesError::StorageError
            })?;
        }

        StorableEncryptedAddPackage::store_multiple(
            &self.db_pool,
            &sender,
            &encrypted_key_packages,
        )
        .await
        .map_err(|e| {
            tracing::warn!("Failed to store last resort key package: {:?}", e);
            QsPublishKeyPackagesError::StorageError
        })?;

        Ok(())
    }

    /// Retrieve a key package for the given client.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_client_key_package(
        &self,
        params: ClientKeyPackageParams,
    ) -> Result<ClientKeyPackageResponse, QsClientKeyPackageError> {
        let ClientKeyPackageParams { sender, client_id } = params;

        let StorableEncryptedAddPackage(encrypted_key_package) =
            StorableEncryptedAddPackage::load(&self.db_pool, &sender, &client_id)
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to load key package: {:?}", e);
                    QsClientKeyPackageError::StorageError
                })?
                .ok_or(QsClientKeyPackageError::NoKeyPackages)?;

        let response = ClientKeyPackageResponse {
            encrypted_key_package,
        };
        Ok(response)
    }

    /// Retrieve a key package for a given client.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_key_package(
        &self,
        params: KeyPackageParams,
    ) -> Result<KeyPackageResponse, QsKeyPackageError> {
        let KeyPackageParams {
            sender,
            friendship_ear_key,
        } = params;

        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection: {:?}", e);
            QsKeyPackageError::StorageError
        })?;

        let encrypted_key_package =
            StorableEncryptedAddPackage::load_user_key_package(&mut connection, &sender)
                .await
                .map_err(|e| {
                    tracing::warn!("Storage provider error: {:?}", e);
                    QsKeyPackageError::StorageError
                })?;

        let key_package = KeyPackageIn::decrypt(&friendship_ear_key, &encrypted_key_package)
            .map_err(|_| QsKeyPackageError::DecryptionError)
            .and_then(|ap| {
                ap.validate(
                    OpenMlsRustCrypto::default().crypto(),
                    ProtocolVersion::default(),
                )
                .map_err(|_| QsKeyPackageError::InvalidKeyPackage)
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
                let encryption_key = decryption_key.encryption_key();
                EncryptionKeyResponse { encryption_key }
            })
            .ok_or(QsEncryptionKeyError::LibraryError)
    }
}
