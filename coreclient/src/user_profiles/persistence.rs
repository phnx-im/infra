// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::QualifiedUserName;
use sqlx::{SqliteExecutor, query, query_as};

use crate::{UserProfile, store::StoreNotifier};

use super::{Asset, DisplayName};

struct SqlUserProfile {
    user_name: QualifiedUserName,
    display_name: Option<DisplayName>,
    profile_picture: Option<Asset>,
}

impl From<SqlUserProfile> for UserProfile {
    fn from(
        SqlUserProfile {
            user_name,
            display_name,
            profile_picture,
        }: SqlUserProfile,
    ) -> Self {
        Self {
            user_name,
            display_name_option: display_name,
            profile_picture_option: profile_picture,
        }
    }
}

impl UserProfile {
    pub async fn load(
        executor: impl SqliteExecutor<'_>,
        user_name: &QualifiedUserName,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            SqlUserProfile,
            r#"SELECT
                user_name AS "user_name: _",
                display_name AS "display_name: _",
                profile_picture AS "profile_picture: _"
            FROM users WHERE user_name = ?"#,
            user_name,
        )
        .fetch_optional(executor)
        .await
        .map(|record| record.map(From::from))
    }

    /// Stores this new [`UserProfile`] if one doesn't already exist.
    pub(crate) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let res = query!(
            "INSERT OR IGNORE INTO users (user_name, display_name, profile_picture)
            VALUES (?, ?, ?)",
            self.user_name,
            self.display_name_option,
            self.profile_picture_option
        )
        .execute(executor)
        .await?;
        if res.rows_affected() > 0 {
            notifier.add(self.user_name.clone());
        }
        Ok(())
    }

    /// Stores this new [`UserProfile`].
    ///
    /// Replaces the existing user profile if one exists.
    pub(crate) async fn upsert(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        query!(
            "INSERT OR REPLACE INTO users (user_name, display_name, profile_picture)
            VALUES (?, ?, ?)",
            self.user_name,
            self.display_name_option,
            self.profile_picture_option,
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
        query!(
            "UPDATE users SET display_name = ?2, profile_picture = ?3 WHERE user_name = ?1",
            self.user_name,
            self.display_name_option,
            self.profile_picture_option
        )
        .execute(executor)
        .await?;
        notifier.update(self.user_name.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use crate::Asset;

    use super::*;

    fn test_profile() -> UserProfile {
        UserProfile::new(
            "alice@localhost".parse().unwrap(),
            Some("Alice".to_string().try_into().unwrap()),
            Some(Asset::Value(vec![1, 2, 3])),
        )
    }

    #[sqlx::test]
    async fn store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let profile = test_profile();

        profile.store(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.set_display_name(Some("Alice In Wonderland".to_string().try_into()?));
        new_profile.set_profile_picture(None);

        // store ignores the new profile if the user already exists
        new_profile.store(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);
        assert_ne!(loaded, new_profile);

        // upsert/load works
        new_profile.upsert(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }

    #[sqlx::test]
    async fn upsert_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let profile = test_profile();

        profile.upsert(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.set_display_name(Some("Alice In Wonderland".to_string().try_into()?));
        new_profile.set_profile_picture(None);

        new_profile.upsert(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }

    #[sqlx::test]
    async fn update_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let profile = test_profile();

        profile.store(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.set_display_name(Some("Alice In Wonderland".to_string().try_into()?));
        new_profile.set_profile_picture(None);

        new_profile.update(&pool, &mut notifier).await?;
        let loaded = UserProfile::load(&pool, &profile.user_name)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }
}
