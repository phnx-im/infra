/*
- ENDPOINT_QS_PUBLISH_KEY_PACKAGES
- ENDPOINT_QS_CLIENT_KEY_PACKAGE
- ENDPOINT_QS_KEY_PACKAGE_BATCH
*/

use mls_assist::{KeyPackage, OpenMlsCryptoProvider, OpenMlsRustCrypto};

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
        KeyPackageBatchTbs, Qs,
    },
};

impl Qs {
    /// Clients publish key packages to the server.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_publish_key_packages<S: QsStorageProvider>(
        storage_provider: &S,
        params: PublishKeyPackagesParams,
    ) -> Result<(), QsPublishKeyPackagesError> {
        let PublishKeyPackagesParams {
            client_id,
            key_packages,
            friendship_ear_key,
        } = params;

        // TODO: Validate the key packages

        let encrypted_key_packages = key_packages
            .into_iter()
            .map(|key_package| {
                key_package
                    .encrypt(&friendship_ear_key)
                    .map_err(|_| QsPublishKeyPackagesError::LibraryError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // TODO: Last resort key package

        storage_provider
            .store_key_packages(&client_id, encrypted_key_packages)
            .await
            .map_err(|_| QsPublishKeyPackagesError::StorageError)?;
        Ok(())
    }

    /// Retrieve a key package for the given client.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_client_key_package<S: QsStorageProvider>(
        storage_provider: &S,
        params: ClientKeyPackageParams,
    ) -> Result<ClientKeyPackageResponse, QsClientKeyPackageError> {
        let ClientKeyPackageParams { client_id } = params;

        let encrypted_key_package = storage_provider
            .load_key_package(&client_id)
            .await
            .ok_or(QsClientKeyPackageError::StorageError)?;

        let response = ClientKeyPackageResponse {
            encrypted_key_package,
        };
        Ok(response)
    }

    /// Retrieve a key package batch for a given client.
    #[tracing::instrument(skip_all, err)]
    pub async fn qs_key_package_batch<S: QsStorageProvider>(
        &self,
        storage_provider: &S,
        params: KeyPackageBatchParams,
    ) -> Result<KeyPackageBatchResponse, QsKeyPackageBatchError> {
        let KeyPackageBatchParams {
            friendship_token,
            friendship_ear_key,
        } = params;

        let encrypted_key_packages = storage_provider
            .load_user_key_packages(&friendship_token)
            .await;

        let key_packages = encrypted_key_packages
            .into_iter()
            .map(|encrypted_key_package| {
                KeyPackage::decrypt(&friendship_ear_key, &encrypted_key_package)
                    .map_err(|_| QsKeyPackageBatchError::DecryptionError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let key_package_refs = key_packages
            .iter()
            .map(|key_package| {
                key_package
                    .hash_ref(OpenMlsRustCrypto::default().crypto())
                    .map_err(|_| QsKeyPackageBatchError::LibraryError)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let key_package_batch_tbs = KeyPackageBatchTbs {
            homeserver_domain: self.fqdn.clone(),
            key_package_refs,
            time_of_signature: TimeStamp::now(),
        };

        let key_package_batch = key_package_batch_tbs
            .sign(&self.signer)
            .map_err(|_| QsKeyPackageBatchError::LibraryError)?;

        let response = KeyPackageBatchResponse {
            key_packages,
            key_package_batch,
        };
        Ok(response)
    }
}
