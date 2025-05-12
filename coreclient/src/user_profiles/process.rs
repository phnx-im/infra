// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    credentials::ClientCredential,
    crypto::{indexed_aead::keys::UserProfileKeyIndex, signatures::signable::Verifiable as _},
    identifiers::QualifiedUserName,
};

use super::{
    IndexedUserProfile, UserProfileValidationError, VerifiableUserProfile, VerifiedUserProfile,
};

pub(crate) struct ExistingUserProfile(Option<IndexedUserProfile>);

impl ExistingUserProfile {
    pub(crate) async fn load(
        executor: impl sqlx::SqliteExecutor<'_>,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Self> {
        let existing_user_profile = IndexedUserProfile::load(executor, user_name).await?;
        Ok(Self(existing_user_profile))
    }

    pub(crate) fn process_decrypted_user_profile(
        self,
        user_profile: VerifiableUserProfile,
        credential: &ClientCredential,
    ) -> Result<PersistableUserProfile, UserProfileValidationError> {
        let VerifiedUserProfile(user_profile) = user_profile.verify(credential.verifying_key())?;
        if let Some(existing_user_profile) = &self.0 {
            if existing_user_profile.user_name != user_profile.user_name {
                return Err(UserProfileValidationError::MismatchingUserName {
                    expected: existing_user_profile.user_name.clone(),
                    actual: user_profile.user_name,
                });
            }
            if existing_user_profile.epoch >= user_profile.epoch {
                return Err(UserProfileValidationError::OutdatedUserProfile {
                    user_name: existing_user_profile.user_name.clone(),
                    epoch: user_profile.epoch,
                });
            }
        }
        Ok(PersistableUserProfile {
            old_profile_index: self.0.map(|profile| profile.decryption_key_index),
            user_profile,
        })
    }

    /// Check if the user profile matches the given key index. Returns false if
    /// the user profile is not present.
    pub(crate) fn matches_index(&self, user_profile_key_index: &UserProfileKeyIndex) -> bool {
        self.0
            .as_ref()
            .map(|profile| profile.decryption_key_index == *user_profile_key_index)
            .unwrap_or(false)
    }
}

pub(crate) struct PersistableUserProfile {
    old_profile_index: Option<UserProfileKeyIndex>,
    user_profile: IndexedUserProfile,
}

impl PersistableUserProfile {
    pub(crate) async fn persist(
        &self,
        executor: impl sqlx::SqliteExecutor<'_>,
        notifier: &mut crate::store::StoreNotifier,
    ) -> sqlx::Result<()> {
        if self.is_update() {
            self.user_profile.update(executor, notifier).await
        } else {
            self.user_profile.store(executor, notifier).await
        }
    }

    pub(crate) fn old_profile_index(&self) -> Option<&UserProfileKeyIndex> {
        self.old_profile_index.as_ref()
    }

    fn is_update(&self) -> bool {
        self.old_profile_index.is_some()
    }
}
