// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Context;
use phnxtypes::{
    crypto::{
        indexed_aead::{
            ciphertexts::{IndexDecryptable, IndexEncryptable},
            keys::UserProfileKey,
        },
        signatures::signable::Signable,
    },
    messages::{client_as_out::GetUserProfileResponse, client_ds::UserProfileKeyUpdateParams},
};

use crate::{
    Contact,
    groups::{Group, ProfileInfo},
    key_stores::indexed_keys::StorableIndexedKey,
    user_profiles::{IndexedUserProfile, UnvalidatedUserProfile, UserProfile},
};

use super::CoreUser;

impl CoreUser {
    pub async fn update_user_profile(
        &self,
        user_profile_content: UserProfile,
    ) -> anyhow::Result<()> {
        let user_profile_key = UserProfileKey::random(self.user_name())?;

        // Phase 1: Store the new user profile key in the database
        let user_profile = self
            .with_transaction_and_notifier(async |transaction, notifier| {
                let current_user_profile_epoch =
                    IndexedUserProfile::load(&mut *transaction, self.user_name())
                        .await?
                        .context("Failed to load own user profile")?
                        .epoch();

                let user_profile = IndexedUserProfile::new(
                    user_profile_content.user_name,
                    current_user_profile_epoch + 1,
                    user_profile_key.index().clone(),
                    user_profile_content.display_name,
                    user_profile_content.profile_picture,
                );

                // Phase 1: Store the user profile and the new key in the database
                user_profile.update(&mut *transaction, notifier).await?;
                user_profile_key.store_own(&mut *transaction).await?;
                Ok(user_profile)
            })
            .await?;

        // Phase 2: Encrypt the user profile
        let signed_user_profile = user_profile.sign(&self.inner.key_store.signing_key)?;
        let encrypted_user_profile = signed_user_profile.encrypt_with_index(&user_profile_key)?;

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
        let mut connection = self.pool().acquire().await?;
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
            client_credential,
        } = profile_info.into();
        let user_name = client_credential.identity().user_name().clone();

        // Phase 1: Check if the profile in the DB is up to date.
        let mut old_user_profile = None;
        if let Some(user_profile) = IndexedUserProfile::load(self.pool(), &user_name).await? {
            if user_profile.decryption_key_index() == user_profile_key.index() {
                return Ok(());
            }
            old_user_profile = Some(user_profile);
        };

        // Phase 2: Fetch the user profile from the server
        let api_client = self.inner.api_clients.get(user_name.domain())?;

        let GetUserProfileResponse {
            encrypted_user_profile,
        } = api_client
            .as_get_user_profile(
                client_credential.identity().clone(),
                user_profile_key.index().clone(),
            )
            .await?;

        let verifiable_user_profile =
            UnvalidatedUserProfile::decrypt_with_index(&user_profile_key, &encrypted_user_profile)?;
        let user_profile: IndexedUserProfile = verifiable_user_profile.validate(
            old_user_profile.as_ref().map(|up| up.epoch()),
            &client_credential,
        )?;

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
            if let Some(old_user_profile) = old_user_profile {
                // Delete the old user profile key
                UserProfileKey::delete(connection, &old_user_profile.decryption_key_index())
                    .await?;
            }
            Ok(())
        })
        .await?;

        Ok(())
    }
}
