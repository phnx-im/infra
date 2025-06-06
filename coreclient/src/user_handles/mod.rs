// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
pub use persistence::UserHandleRecord;
use phnxcommon::{credentials::keys::HandleSigningKey, identifiers::UserHandle};
use tracing::error;

use crate::{clients::CoreUser, store::StoreResult};

mod persistence;

impl CoreUser {
    /// Registers a new user handle on the server and adds it locally.
    ///
    /// Returns `true` on success, or `false` if the handle was already present.
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

        if let Err(error) = record.store(self.pool()).await {
            error!(%error, "failed to store user handle; rollback on the server");
            api_client
                .as_delete_handle(hash, &record.signing_key)
                .await
                .inspect_err(|error| {
                    error!(%error, "failed to delete user handle after error");
                })
                .ok();
            return Err(error.into());
        }

        // TODO: Publish connection packages for a handle

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
