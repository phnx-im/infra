// SPDX-FileCopyrightTextj 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{identifiers::QualifiedUserName, messages::client_as_out::EncryptedUserProfile};

use crate::errors::StorageError;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct UserRecord {
    user_name: QualifiedUserName,
    encrypted_user_profile: EncryptedUserProfile,
}

impl UserRecord {
    fn new(user_name: QualifiedUserName, encrypted_user_profile: EncryptedUserProfile) -> Self {
        Self {
            user_name,
            encrypted_user_profile,
        }
    }

    pub(super) async fn new_and_store(
        connection: impl sqlx::PgExecutor<'_>,
        user_name: &QualifiedUserName,
        encrypted_user_profile: &EncryptedUserProfile,
    ) -> Result<Self, StorageError> {
        let user_record = Self::new(user_name.clone(), encrypted_user_profile.clone());
        user_record.store(connection).await?;
        Ok(user_record)
    }

    #[cfg(test)]
    pub(super) fn user_name(&self) -> &QualifiedUserName {
        &self.user_name
    }

    pub(super) fn into_encrypted_user_profile(self) -> EncryptedUserProfile {
        self.encrypted_user_profile
    }

    pub(super) fn set_user_profile(&mut self, encrypted_user_profile: EncryptedUserProfile) {
        self.encrypted_user_profile = encrypted_user_profile;
    }
}

pub(crate) mod persistence {
    use phnxtypes::identifiers::QualifiedUserName;
    use sqlx::{PgExecutor, query, query_scalar};

    use crate::errors::StorageError;

    use super::UserRecord;

    impl UserRecord {
        /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
        /// exists for the given UserId.
        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
            user_name: &QualifiedUserName,
        ) -> Result<Option<UserRecord>, StorageError> {
            let encrypted_user_profile = query_scalar!(
                r#"SELECT
                    encrypted_user_profile AS "encrypted_user_profile: _"
                FROM as_user_records
                WHERE user_name = $1"#,
                user_name.to_string(),
            )
            .fetch_optional(connection)
            .await?;
            Ok(encrypted_user_profile.map(|profile| UserRecord::new(user_name.clone(), profile)))
        }

        /// Update the AsUserRecord for a given UserId.
        pub(crate) async fn update(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            query!(
                "UPDATE as_user_records
                SET encrypted_user_profile = $1
                WHERE user_name = $2",
                self.encrypted_user_profile as _,
                self.user_name.to_string()
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        /// Create a new user with the given user name. If a user with the given user
        /// name already exists, an error is returned.
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            query!(
                "INSERT INTO as_user_records (user_name, encrypted_user_profile) VALUES ($1, $2)",
                self.user_name.to_string(),
                self.encrypted_user_profile as _
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        /// Deletes the AsUserRecord for a given UserId. Returns true if a AsUserRecord
        /// was deleted, false if no AsUserRecord existed for the given UserId.
        ///
        /// The storage provider must also delete the following:
        ///  - All clients of the user
        ///  - All enqueued messages for the respective clients
        ///  - All key packages for the respective clients
        pub(in crate::auth_service) async fn delete(
            connection: impl PgExecutor<'_>,
            user_name: &QualifiedUserName,
        ) -> Result<(), sqlx::Error> {
            // The database cascades the delete to the clients and their connection packages.
            query!(
                "DELETE FROM as_user_records WHERE user_name = $1",
                user_name.to_string()
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) mod tests {
        use phnxtypes::messages::client_as_out::EncryptedUserProfile;
        use sqlx::PgPool;
        use uuid::Uuid;

        use super::*;

        pub(crate) async fn store_random_user_record(pool: &PgPool) -> anyhow::Result<UserRecord> {
            let user_name: QualifiedUserName = format!("{}@example.com", Uuid::new_v4()).parse()?;
            let encrypted_user_profile = EncryptedUserProfile::dummy();
            let record = UserRecord {
                user_name,
                encrypted_user_profile,
            };
            record.store(pool).await?;
            Ok(record)
        }

        #[sqlx::test]
        async fn load(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;

            let loaded = UserRecord::load(&pool, &user_record.user_name)
                .await?
                .expect("missing user record");
            assert_eq!(loaded, user_record);

            Ok(())
        }

        #[sqlx::test]
        async fn delete(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;

            let loaded = UserRecord::load(&pool, &user_record.user_name)
                .await?
                .expect("missing user record");
            assert_eq!(loaded, user_record);

            UserRecord::delete(&pool, &user_record.user_name).await?;

            let loaded = UserRecord::load(&pool, &user_record.user_name).await?;
            assert!(loaded.is_none());

            Ok(())
        }
    }
}
