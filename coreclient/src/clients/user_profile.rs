// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::crypto::ear::EarEncryptable;

use crate::UserProfile;

use super::CoreUser;

impl CoreUser {
    pub async fn update_user_profile(&self, user_profile: &UserProfile) -> anyhow::Result<()> {
        let mut notifier = self.store_notifier();

        // Phase 1: Store the user profile update in the database
        user_profile.update(self.pool(), &mut notifier).await?;

        // Phase 2: Encrypt the user profile
        let encrypted_user_profile =
            user_profile.encrypt(&self.inner.key_store.user_profile_key)?;

        // Phase 3: Send the updated profile to the server
        let api_client = self.inner.api_clients.default_client()?;

        api_client
            .as_update_user_profile(
                self.as_client_id(),
                &self.inner.key_store.signing_key,
                encrypted_user_profile,
            )
            .await?;

        // Phase 4: Notify the store about the update
        // TODO: Send user profile update notification.

        Ok(())
    }
}
