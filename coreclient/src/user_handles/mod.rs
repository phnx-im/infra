// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
pub use persistence::UserHandleRecord;
use phnxcommon::{
    credentials::keys::HandleSigningKey,
    crypto::signatures::signable::Signable,
    identifiers::UserHandle,
    messages::{
        MlsInfraVersion,
        client_as::{ConnectionPackage, ConnectionPackageTbs},
    },
    time::ExpirationData,
};
use tracing::error;

use crate::{
    clients::{CONNECTION_PACKAGE_EXPIRATION, CONNECTION_PACKAGES, CoreUser},
    key_stores::MemoryUserKeyStore,
    store::StoreResult,
};

mod persistence;

impl CoreUser {
    /// Registers a new user handle on the server and adds it locally.
    ///
    /// Returns a handle record on success, or `None` if the handle was already present.
    pub(crate) async fn add_user_handle(
        &self,
        handle: &UserHandle,
    ) -> StoreResult<Option<UserHandleRecord>> {
        let signing_key = HandleSigningKey::generate()?;
        let hash = handle.hash()?;

        let api_client = self.api_client()?;
        let created = api_client
            .as_create_handle(handle, hash, &signing_key)
            .await?;
        if !created {
            return Ok(None);
        }

        let record = UserHandleRecord::new(handle.clone(), hash, signing_key);

        let rollback = async || {
            api_client
                .as_delete_handle(record.hash, &record.signing_key)
                .await
                .inspect_err(|error| {
                    error!(%error, "failed to delete user handle in rollback");
                })
                .ok();
        };

        if let Err(error) = record.store(self.pool()).await {
            error!(%error, "failed to store user handle; rollback on the server");
            rollback().await;
            return Err(error.into());
        }

        // Publish connection packages
        let connection_packages = generate_connection_packages(self.key_store())?;
        if let Err(error) = api_client
            .as_publish_connection_packages_for_handle(
                hash,
                connection_packages,
                &record.signing_key,
            )
            .await
        {
            error!(%error, "failed to publish connection packages; rollback on the server");
            rollback().await;
            return Err(error.into());
        }

        Ok(Some(record))
    }

    /// Deletes the user handle on the server and removes it locally.
    pub(crate) async fn remove_user_handle(&self, handle: &UserHandle) -> StoreResult<()> {
        let mut txn = self.pool().begin().await?;
        let record = UserHandleRecord::load(txn.as_mut(), handle)
            .await?
            .context("no user handle found")?;
        let api_client = self.api_client()?;
        api_client
            .as_delete_handle(record.hash, &record.signing_key)
            .await?;
        UserHandleRecord::delete(txn.as_mut(), handle).await?;
        txn.commit().await?;
        Ok(())
    }
}

fn generate_connection_packages(
    key_store: &MemoryUserKeyStore,
) -> anyhow::Result<Vec<ConnectionPackage>> {
    // TODO: For now, we use the same ConnectionDecryptionKey for all
    // connection packages.
    let mut connection_packages = Vec::with_capacity(CONNECTION_PACKAGES);
    for _ in 0..CONNECTION_PACKAGES {
        let lifetime = ExpirationData::new(CONNECTION_PACKAGE_EXPIRATION);
        let connection_package_tbs = ConnectionPackageTbs::new(
            MlsInfraVersion::default(),
            key_store.connection_decryption_key.encryption_key().clone(),
            lifetime,
            key_store.signing_key.credential().clone(),
        );
        let connection_package = connection_package_tbs.sign(&key_store.signing_key)?;
        connection_packages.push(connection_package);
    }
    Ok(connection_packages)
}
