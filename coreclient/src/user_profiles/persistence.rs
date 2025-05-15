// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{crypto::indexed_aead::keys::UserProfileKeyIndex, identifiers::AsClientId};
use sqlx::{SqliteExecutor, query, query_as};

use crate::store::StoreNotifier;

use super::{Asset, IndexedUserProfile, UserProfile, display_name::BaseDisplayName};

impl IndexedUserProfile {
    /// Stores this [`BaseIndexedUserProfile`].
    ///
    /// Will return an error if there already exists a user profile with the same user id.
    pub(super) async fn store(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let uuid = self.client_id.client_id();
        let domain = self.client_id.domain();
        let epoch = self.epoch as i64;
        query!(
            "INSERT INTO users (
                as_client_uuid,
                as_domain,
                epoch,
                decryption_key_index,
                display_name,
                profile_picture
            ) VALUES (?, ?, ?, ?, ?, ?)",
            uuid,
            domain,
            epoch,
            self.decryption_key_index,
            self.display_name,
            self.profile_picture,
        )
        .execute(executor)
        .await?;
        notifier.update(self.client_id.clone());
        Ok(())
    }

    /// Update the user's display name and profile picture in the database.
    pub(crate) async fn update(
        &self,
        executor: impl SqliteExecutor<'_>,
        notifier: &mut StoreNotifier,
    ) -> sqlx::Result<()> {
        let uuid = self.client_id.client_id();
        let domain = self.client_id.domain();
        let epoch = self.epoch as i64;
        query!(
            "UPDATE users SET
                epoch = ?3,
                decryption_key_index = ?4,
                display_name = ?5,
                profile_picture = ?6
            WHERE as_client_uuid = ?1 AND as_domain = ?2",
            uuid,
            domain,
            epoch,
            self.decryption_key_index,
            self.display_name,
            self.profile_picture
        )
        .execute(executor)
        .await?;
        notifier.update(self.client_id.clone());
        Ok(())
    }
}

struct SqlUser {
    epoch: u64,
    decryption_key_index: UserProfileKeyIndex,
    display_name: BaseDisplayName<true>,
    profile_picture: Option<Asset>,
}

impl From<(AsClientId, SqlUser)> for IndexedUserProfile {
    fn from(
        (
            client_id,
            SqlUser {
                epoch,
                decryption_key_index,
                display_name,
                profile_picture,
            },
        ): (AsClientId, SqlUser),
    ) -> Self {
        Self {
            client_id,
            epoch,
            decryption_key_index,
            display_name,
            profile_picture,
        }
    }
}

impl IndexedUserProfile {
    pub(crate) async fn load(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<Option<Self>> {
        let uuid = client_id.client_id();
        let domain = client_id.domain();
        query_as!(
            SqlUser,
            r#"SELECT
                epoch AS "epoch: _",
                decryption_key_index AS "decryption_key_index: _",
                display_name AS "display_name: _",
                profile_picture AS "profile_picture: _"
            FROM users
            WHERE as_client_uuid = ? AND as_domain = ?"#,
            uuid,
            domain,
        )
        .fetch_optional(executor)
        .await
        .map(|res| res.map(|user| (client_id.clone(), user).into()))
    }
}

impl UserProfile {
    pub async fn load(
        executor: impl SqliteExecutor<'_>,
        client_id: &AsClientId,
    ) -> sqlx::Result<Option<Self>> {
        IndexedUserProfile::load(executor, client_id)
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
        let client_id = AsClientId::random("localhost".parse().unwrap());
        let user_profile_key = UserProfileKey::random(&client_id).unwrap();
        let user_profile = IndexedUserProfile {
            client_id,
            epoch: 0,
            decryption_key_index: user_profile_key.index().clone(),
            display_name: "Alice".parse().unwrap(),
            profile_picture: Some(Asset::Value(vec![1, 2, 3])),
        };
        (user_profile, user_profile_key)
    }

    #[sqlx::test]
    async fn store_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let (profile, key) = test_profile();

        key.store(&pool).await?;

        profile.store(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.client_id)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.display_name = "Alice In Wonderland".parse()?;
        new_profile.profile_picture = None;

        // store again doesn't work
        let store_err = new_profile
            .store(&pool, &mut notifier)
            .await
            .expect_err("profile does not exist");
        assert!(matches!(store_err, sqlx::Error::Database(_)));

        Ok(())
    }

    #[sqlx::test]
    async fn update_load(pool: SqlitePool) -> anyhow::Result<()> {
        let mut notifier = StoreNotifier::noop();

        let (profile, key) = test_profile();
        key.store(&pool).await?;

        profile.store(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.client_id)
            .await?
            .expect("profile exists");
        assert_eq!(loaded, profile);

        let mut new_profile = profile.clone();
        new_profile.display_name = "Alice In Wonderland".parse()?;
        new_profile.profile_picture = None;

        new_profile.update(&pool, &mut notifier).await?;
        let loaded = IndexedUserProfile::load(&pool, &profile.client_id)
            .await?
            .expect("profile exists");
        assert_ne!(loaded, profile);
        assert_eq!(loaded, new_profile);

        Ok(())
    }
}
