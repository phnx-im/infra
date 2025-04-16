// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use phnxtypes::{
    crypto::ear::{EarDecryptable, EarEncryptable},
    messages::{client_as_out::GetUserProfileResponse, client_ds::UserProfileKeyUpdateParams},
};

use crate::{
    UserProfile,
    groups::{Group, ProfileInfo},
    key_stores::indexed_keys::UserProfileKey,
};

use super::CoreUser;

impl CoreUser {
    pub async fn update_user_profile(&self, user_profile: &UserProfile) -> anyhow::Result<()> {
        let mut notifier = self.store_notifier();

        // Phase 1: Store the user profile update in the database
        user_profile.update(self.pool(), &mut notifier).await?;

        notifier.notify();

        let mut connection = self.pool().acquire().await?;

        let user_profile_key = UserProfileKey::load_own(&mut *connection).await?;

        // Phase 2: Encrypt the user profile
        let encrypted_user_profile = user_profile.encrypt(&user_profile_key)?;

        // Phase 3: Send the updated profile to the server
        let api_client = self.inner.api_clients.default_client()?;

        api_client
            .as_update_user_profile(
                self.as_client_id(),
                &self.inner.key_store.signing_key,
                encrypted_user_profile,
            )
            .await?;

        // Phase 4: Send a notification to all groups
        let groups_ids = Group::load_all_group_ids(&mut *connection).await?;
        for group_id in groups_ids {
            let group = Group::load(&mut *connection, &group_id)
                .await?
                .context("Failed to load group")?;
            let own_index = group.own_index();
            let user_profile_key = user_profile_key.encrypt(group.identity_link_wrapper_key())?;
            let params = UserProfileKeyUpdateParams {
                group_id,
                sender_index: own_index,
                epoch: group.epoch(),
                user_profile_key: user_profile_key.clone(),
            };
            api_client
                .ds_user_profile_key_update(
                    params,
                    group.leaf_signer(),
                    group.group_state_ear_key(),
                )
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn fetch_user_profile(
        &self,
        profile_info: impl Into<ProfileInfo>,
    ) -> anyhow::Result<UserProfile> {
        let profile_info = profile_info.into();
        let api_client = self
            .inner
            .api_clients
            .get(profile_info.member_id.user_name().domain())?;

        let GetUserProfileResponse {
            encrypted_user_profile,
        } = api_client
            .as_get_user_profile(profile_info.member_id)
            .await?;

        let user_profile =
            UserProfile::decrypt(&profile_info.user_profile_key, &encrypted_user_profile)?;

        Ok(user_profile)
    }
}
