// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::ServerRegistration;
use phnxtypes::{crypto::OpaqueCiphersuite, identifiers::QualifiedUserName};

use crate::errors::StorageError;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct UserRecord {
    user_name: QualifiedUserName,
    password_file: ServerRegistration<OpaqueCiphersuite>,
}

impl UserRecord {
    fn new(
        user_name: QualifiedUserName,
        password_file: ServerRegistration<OpaqueCiphersuite>,
    ) -> Self {
        Self {
            user_name,
            password_file,
        }
    }

    pub(super) async fn new_and_store(
        connection: impl sqlx::PgExecutor<'_>,
        user_name: &QualifiedUserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
    ) -> Result<Self, StorageError> {
        let user_record = Self::new(user_name.clone(), opaque_record.clone());
        user_record.store(connection).await?;
        Ok(user_record)
    }

    #[cfg(test)]
    pub(super) fn user_name(&self) -> &QualifiedUserName {
        &self.user_name
    }

    pub(super) fn into_password_file(self) -> ServerRegistration<OpaqueCiphersuite> {
        self.password_file
    }
}

pub(crate) mod persistence {
    use phnxtypes::{
        codec::PhnxCodec,
        identifiers::{QualifiedUserName, UserName},
    };
    use sqlx::PgExecutor;

    use crate::errors::StorageError;

    use super::UserRecord;

    impl UserRecord {
        /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
        /// exists for the given UserId.
        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
            user_name: &QualifiedUserName,
        ) -> Result<Option<UserRecord>, StorageError> {
            sqlx::query!(
                r#"SELECT user_name as "user_name: UserName", password_file
                FROM as_user_records
                WHERE user_name = $1"#,
                user_name.to_string(),
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                let password_file = PhnxCodec::from_slice(&record.password_file)?;
                Ok(UserRecord::new(user_name.clone(), password_file))
            })
            .transpose()
        }

        /// Create a new user with the given user name. If a user with the given user
        /// name already exists, an error is returned.
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            let password_file_bytes = PhnxCodec::to_vec(&self.password_file)?;
            sqlx::query!(
                "INSERT INTO as_user_records (user_name, password_file) VALUES ($1, $2)",
                self.user_name.to_string(),
                password_file_bytes,
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
            sqlx::query!(
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
        use opaque_ke::{
            ClientRegistration, ClientRegistrationFinishParameters, ServerRegistration, ServerSetup,
        };
        use sqlx::PgPool;
        use uuid::Uuid;

        use super::*;

        pub(crate) async fn store_random_user_record(pool: &PgPool) -> anyhow::Result<UserRecord> {
            let user_name: QualifiedUserName = format!("{}@example.com", Uuid::new_v4()).parse()?;
            let password = b"password";

            let mut rng = rand::thread_rng();
            let server_setup = ServerSetup::new(&mut rng);
            let client_registration_start_result =
                ClientRegistration::start(&mut rng, password).unwrap();
            let server_registration_start_result = ServerRegistration::start(
                &server_setup,
                client_registration_start_result.message,
                user_name.to_string().as_bytes(),
            )
            .unwrap();
            let client_registration_finish_result = client_registration_start_result
                .state
                .finish(
                    &mut rng,
                    password,
                    server_registration_start_result.message,
                    ClientRegistrationFinishParameters::default(),
                )
                .unwrap();
            let password_file =
                ServerRegistration::finish(client_registration_finish_result.message);

            let record = UserRecord {
                user_name,
                password_file,
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
