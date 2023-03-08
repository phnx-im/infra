/*
- ENDPOINT_QS_PUBLISH_KEY_PACKAGES
- ENDPOINT_QS_CLIENT_KEY_PACKAGE
- ENDPOINT_QS_KEY_PACKAGE_BATCH
*/

use crate::{
    crypto::signatures::signable::Signable,
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
            add_packages,
            friendship_ear_key,
        } = params;

        // TODO: Validate the key packages after decrypting them wit the
        // friendship EAR key
        let _ = friendship_ear_key;

        // TODO: Last resort key package

        storage_provider
            .store_key_packages(&client_id, add_packages)
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
        // TODO: We dercypt the key packages on the fly, compute their
        // KeyPackageRef and sign the batch
        let _ = friendship_ear_key;
        let key_package_refs = Vec::new();

        let key_package_batch_tbs = KeyPackageBatchTbs {
            homeserver_domain: self.fqdn.clone(),
            key_package_refs,
            time_of_signature: TimeStamp::now(),
        };

        let key_package_batch = key_package_batch_tbs
            .sign(&self.signer)
            .map_err(|_| QsKeyPackageBatchError::LibraryError)?;

        let response = KeyPackageBatchResponse {
            encrypted_key_packages,
            key_package_batch,
        };
        Ok(response)
    }
}
