// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use persistence::UserHandleRecord;
use phnxcommon::{credentials::keys::HandleSigningKey, identifiers::UserHandle};
use tracing::error;

use crate::{clients::CoreUser, store::StoreResult};

mod persistence;

impl CoreUser {
    pub(crate) async fn user_handles(&self) -> StoreResult<Vec<UserHandle>> {
        Ok(UserHandleRecord::load_all_handles(self.pool()).await?)
    }

    /// Registers a new user handle on the server and adds it locally.
    ///
    /// Returns `true` on success, or `false` if the handle was already present.
    pub(crate) async fn add_user_handle(&self, handle: &UserHandle) -> StoreResult<bool> {
        let signing_key = HandleSigningKey::generate()?;
        let hash = handle.hash()?;

        let api_client = self.api_client()?;
        let created = api_client
            .as_create_handle(handle, hash, &signing_key)
            .await?;
        if !created {
            return Ok(false);
        }

        if let Err(error) = UserHandleRecord::store(self.pool(), handle, &hash, &signing_key).await
        {
            error!(%error, "failed to store user handle; rollback on the server");
            api_client
                .as_delete_handle(hash, &signing_key)
                .await
                .inspect_err(|error| {
                    error!(%error, "failed to delete user handle after error");
                })
                .ok();
            return Err(error.into());
        }

        Ok(true)
    }

    /// Deletes the user handle on the server and removes it locally.
    pub(crate) async fn remove_user_handle(&self, handle: &UserHandle) -> StoreResult<()> {
        let mut txn = self.pool().begin().await?;
        let record = UserHandleRecord::load(txn.as_mut(), handle)
            .await?
            .context("no user handle found")?;
        let api_client = self.api_client()?;
        api_client
            .as_delete_handle(record.hash, &record.signature_key)
            .await?;
        UserHandleRecord::delete(txn.as_mut(), handle).await?;
        txn.commit().await?;
        Ok(())
    }
}
