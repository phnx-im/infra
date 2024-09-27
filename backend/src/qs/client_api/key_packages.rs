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

use crate::qs::{
    add_package::StorableEncryptedAddPackage,
    client_id_decryption_key::StorableClientIdDecryptionKey, signing_key::StorableQsSigningKey, Qs,
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
            add_packages,
            friendship_ear_key,
        } = params;

        let mut encrypted_add_packages = vec![];
        let mut last_resort_add_package = None;
        for add_package in add_packages {
            let verified_add_package: AddPackage = add_package
                .validate(
                    OpenMlsRustCrypto::default().crypto(),
                    ProtocolVersion::default(),
                )
                .map_err(|_| QsPublishKeyPackagesError::InvalidKeyPackage)?;

            let is_last_resort = verified_add_package.key_package().last_resort();

            let eap = verified_add_package
                .encrypt(&friendship_ear_key)
                .map_err(|_| QsPublishKeyPackagesError::LibraryError)?;

            if is_last_resort {
                last_resort_add_package = Some(eap);
            } else {
                encrypted_add_packages.push(eap);
            }
        }

        if let Some(last_resort_add_package) = last_resort_add_package {
            StorableEncryptedAddPackage::store_last_resort(
                &self.db_pool,
                &sender,
                &last_resort_add_package,
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
            &encrypted_add_packages,
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

        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection: {:?}", e);
            QsClientKeyPackageError::StorageError
        })?;

        let StorableEncryptedAddPackage(encrypted_key_package) =
            StorableEncryptedAddPackage::load(&mut connection, &sender, &client_id)
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

    /// Retrieve a key package batch for a given client.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_key_package_batch(
        &self,
        params: KeyPackageBatchParams,
    ) -> Result<KeyPackageBatchResponse, QsKeyPackageBatchError> {
        let KeyPackageBatchParams {
            sender,
            friendship_ear_key,
        } = params;

        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection: {:?}", e);
            QsKeyPackageBatchError::StorageError
        })?;

        let encrypted_key_packages =
            StorableEncryptedAddPackage::load_user_key_packages(&mut connection, &sender)
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

        let signing_key = StorableQsSigningKey::load(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to load signing key: {:?}", e);
                QsKeyPackageBatchError::StorageError
            })?
            .ok_or(QsKeyPackageBatchError::LibraryError)?;

        let key_package_batch = key_package_batch_tbs
            .sign(&*signing_key)
            .map_err(|_| QsKeyPackageBatchError::LibraryError)?;

        let response = KeyPackageBatchResponse {
            add_packages,
            key_package_batch,
        };
        Ok(response)
    }

    /// Retrieve the verifying key of this QS
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_verifying_key(
        &self,
    ) -> Result<VerifyingKeyResponse, QsVerifyingKeyError> {
        StorableQsSigningKey::load(&self.db_pool)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to load signing key: {:?}", e);
                QsVerifyingKeyError::StorageError
            })?
            .map(|signing_key| {
                let verifying_key = signing_key.verifying_key().clone();
                VerifyingKeyResponse { verifying_key }
            })
            .ok_or(QsVerifyingKeyError::LibraryError)
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
