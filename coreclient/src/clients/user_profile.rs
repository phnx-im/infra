// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use phnxtypes::{
    crypto::indexed_aead::{
        ciphertexts::{IndexDecryptable, IndexEncryptable},
        keys::UserProfileKey,
    },
    messages::{client_as_out::GetUserProfileResponse, client_ds::UserProfileKeyUpdateParams},
};

use crate::{
    Contact,
    groups::{Group, ProfileInfo},
    key_stores::indexed_keys::StorableIndexedKey,
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

        let user_profile_key = UserProfileKey::random(self.user_name())?;
        let user_profile = IndexedUserProfile::new(
            user_profile_content.user_name,
            user_profile_key.index().clone(),
            user_profile_content.display_name,
            user_profile_content.profile_picture,
        );

        // Phase 1: Store the user profile and the new key in the database
        user_profile.update(&mut *connection, &mut notifier).await?;
        user_profile_key.store_own(&mut connection).await?;

        let own_key = UserProfileKey::load_own(connection.as_mut()).await?;
        assert_eq!(own_key.index(), user_profile_key.index());

        notifier.notify();

        // Phase 2: Encrypt the user profile
        let encrypted_user_profile = user_profile.encrypt_with_index(&user_profile_key)?;

        // Phase 3: Stage the updated profile on the server
        let api_client = self.inner.api_clients.default_client()?;

        api_client
            .as_stage_user_profile(
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

        // Phase 5: Merge the user profile on the server
        api_client
            .as_merge_user_profile(self.as_client_id(), &self.inner.key_store.signing_key)
            .await?;

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
        let user_name = member_id.user_name().clone();

        // Phase 1: Check if the profile in the DB is up to date.
        let mut connection = self.pool().acquire().await?;
        let mut old_user_profile_key_index = None;
        if let Some(user_profile) =
            IndexedUserProfile::load(connection.as_mut(), &user_name).await?
        {
            if user_profile.decryption_key_index() == user_profile_key.index() {
                return Ok(());
            }
            old_user_profile_key_index = Some(user_profile.decryption_key_index().clone());
        };
        drop(connection);

        // Phase 2: Fetch the user profile from the server
        let api_client = self.inner.api_clients.get(user_name.domain())?;

        let GetUserProfileResponse {
            encrypted_user_profile,
        } = api_client
            .as_get_user_profile(member_id, user_profile_key.index().clone())
            .await?;

        let user_profile =
            IndexedUserProfile::decrypt_with_index(&user_profile_key, &encrypted_user_profile)?;

        // Phase 3: Store the user profile and key in the database
        self.with_transaction_and_notifier(async |connection, notifier| {
            user_profile_key.store(&mut *connection).await?;
            user_profile.upsert(&mut *connection, notifier).await?;
            Contact::update_user_profile_key_index(
                &mut *connection,
                &user_name,
                user_profile_key.index(),
            )
            .await?;
            if let Some(old_user_profile_key_index) = old_user_profile_key_index {
                // Delete the old user profile key
                UserProfileKey::delete(connection, &old_user_profile_key_index).await?;
            }
            Ok(())
        })
        .await?;

        Ok(())
    }
}
