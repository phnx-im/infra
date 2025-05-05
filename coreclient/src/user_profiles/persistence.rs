// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::QualifiedUserName;
use sqlx::{SqliteExecutor, query, query_as};

use crate::store::StoreNotifier;

use super::{IndexedUserProfile, UserProfile};

impl IndexedUserProfile {
    pub async fn load(
        executor: impl SqliteExecutor<'_>,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            IndexedUserProfile,
            r#"SELECT
                user_name AS "user_name: _",
                epoch AS "epoch: _",
                decryption_key_index AS "decryption_key_index: _",
                display_name AS "display_name: _",
                profile_picture AS "profile_picture: _"
            FROM users WHERE user_name = ?"#,
            user_name,
        )
        .fetch_optional(executor)
        .await
    }

    /// Stores this new [`UserProfile`].
    ///
    /// Replaces the existing user profile if one exists.
    pub(crate) async fn upsert(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let epoch = self.epoch as i64;
        query!(
            "INSERT OR REPLACE INTO users (user_name, epoch, decryption_key_index, display_name, profile_picture)
            VALUES (?, ?, ?, ?, ?)",
            self.user_name,
            epoch,
            self.decryption_key_index,
            self.display_name,
            self.profile_picture,
        )
        .execute(executor)
        .await?;
        notifier.update(self.user_name.clone());
        Ok(())
    }

    /// Update the user's display name and profile picture in the database. To store a new profile,
    /// use [`register_as_conversation_participant`] instead.
    pub(crate) async fn update(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let epoch = self.epoch as i64;
        query!(
            "UPDATE users SET epoch = ?2, decryption_key_index = ?3, display_name = ?4, profile_picture = ?5 WHERE user_name = ?1",
            self.user_name,
            epoch,
            self.decryption_key_index,
            self.display_name,
            self.profile_picture
        )
        .execute(executor)
        .await?;
        notifier.update(self.user_name.clone());
        Ok(())
    }
}

impl UserProfile {
    pub async fn load(
        executor: impl SqliteExecutor<'_>,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        IndexedUserProfile::load(executor, user_name)
            .await
            .map(|res| res.map(From::from))
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::crypto::indexed_aead::keys::UserProfileKey;
    use sqlx::SqlitePool;

    use crate::{Asset, key_stores::indexed_keys::StorableIndexedKey};

    use super::*;

    fn test_profile() -> (IndexedUserProfile, UserProfileKey) {
        let user_name = "alice@localhost".parse().unwrap();
        let user_profile_key = UserProfileKey::random(&user_name).unwrap();
        let user_profile = IndexedUserProfile::new(
            user_name,
            0,
            user_profile_key.index().clone(),
            Some("Alice".to_string().try_into().unwrap()),
            Some(Asset::Value(vec![1, 2, 3])),
        );
        (user_profile, user_profile_key)
    }

    #[sqlx::test]
    async fn store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let (profile, key) = test_profile();

        key.store(&pool).await?;

        profile.upsert(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.display_name = Some("Alice In Wonderland".to_string().try_into()?);
        new_profile.profile_picture = None;

        // upsert/load works
        new_profile.upsert(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }

    #[sqlx::test]
    async fn upsert_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let (profile, key) = test_profile();
        key.store(&pool).await?;

        profile.upsert(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.display_name = Some("Alice In Wonderland".to_string().try_into()?);
        new_profile.profile_picture = None;

        new_profile.upsert(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }

    #[sqlx::test]
    async fn update_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let (profile, key) = test_profile();
        key.store(&pool).await?;

        profile.upsert(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.display_name = Some("Alice In Wonderland".to_string().try_into()?);
        new_profile.profile_picture = None;

        new_profile.update(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }
}
