// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use phnxtypes::{
    crypto::ear::{EarDecryptable, EarEncryptable},
    messages::{client_as_out::GetUserProfileResponse, client_ds::UserProfileKeyUpdateParams},
};

use crate::{
    groups::{Group, ProfileInfo},
    key_stores::indexed_keys::UserProfileKey,
    user_profiles::{IndexedUserProfile, UserProfile},
};

use super::CoreUser;

impl CoreUser {
    pub async fn update_user_profile(
        &self,
        user_profile_content: UserProfile,
    ) -> anyhow::Result<()> {
        let mut notifier = self.store_notifier();
        let mut connection = self.pool().acquire().await?;

        let user_profile_key = UserProfileKey::load_own(&mut *connection).await?;
        let user_profile = IndexedUserProfile::new(
            user_profile_content.user_name,
            user_profile_key.index().clone(),
            user_profile_content.display_name,
            user_profile_content.profile_picture,
        );

        // Phase 1: Store the user profile update in the database
        user_profile.update(&mut *connection, &mut notifier).await?;

        notifier.notify();

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
        let own_user_name = self.user_name();
        let groups_ids = Group::load_all_group_ids(&mut connection).await?;
        for group_id in groups_ids {
            let group = Group::load(&mut connection, &group_id)
                .await?
                .context("Failed to load group")?;
            let own_index = group.own_index();
            let user_profile_key =
                user_profile_key.encrypt(group.identity_link_wrapper_key(), own_user_name)?;
            let params = UserProfileKeyUpdateParams {
                group_id,
                sender_index: own_index,
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

    pub(crate) async fn fetch_and_store_user_profile(
        &self,
        profile_info: impl Into<ProfileInfo>,
    ) -> anyhow::Result<()> {
        let ProfileInfo {
            user_profile_key,
            member_id,
        } = profile_info.into();
        // TODO: This check will be enabled with Phase 3 of the user profile feature
        // // Phase 1: Check if the profile in the DB is up to date.
        // let mut connection = self.pool().acquire().await?;
        // if let Some(user_profile) =
        //     UserProfile::load(connection.as_mut(), member_id.user_name()).await?
        // {
        //     if user_profile.decryption_key_index() == user_profile_key.index() {
        //         return Ok(());
        //     }
        // }
        // drop(connection);

        // Phase 2: Fetch the user profile from the server
        let api_client = self.inner.api_clients.get(member_id.user_name().domain())?;

        let GetUserProfileResponse {
            encrypted_user_profile,
        } = api_client.as_get_user_profile(member_id).await?;

        let user_profile = IndexedUserProfile::decrypt(&user_profile_key, &encrypted_user_profile)?;

        // Phase 3: Store the user profile and key in the database
        self.with_transaction_and_notifier(async |connection, notifier| {
            user_profile_key.store(&mut *connection).await?;
            user_profile.upsert(connection, notifier).await?;
            Ok(())
        })
        .await?;

        Ok(())
    }
}
