// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::ServerRegistration;
use phnxtypes::{
    crypto::OpaqueCiphersuite, identifiers::QualifiedUserName,
    messages::client_as_out::EncryptedUserProfile,
};

use crate::errors::StorageError;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct UserRecord {
    user_name: QualifiedUserName,
    password_file: ServerRegistration<OpaqueCiphersuite>,
    encrypted_user_profile: EncryptedUserProfile,
}

impl UserRecord {
    fn new(
        user_name: QualifiedUserName,
        password_file: ServerRegistration<OpaqueCiphersuite>,
        encrypted_user_profile: EncryptedUserProfile,
    ) -> Self {
        Self {
            user_name,
            password_file,
            encrypted_user_profile,
        }
    }

    pub(super) async fn new_and_store(
        connection: impl sqlx::PgExecutor<'_>,
        user_name: &QualifiedUserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
        encrypted_user_profile: &EncryptedUserProfile,
    ) -> Result<Self, StorageError> {
        let user_record = Self::new(
            user_name.clone(),
            opaque_record.clone(),
            encrypted_user_profile.clone(),
        );
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

    pub(super) fn into_encrypted_user_profile(self) -> EncryptedUserProfile {
        self.encrypted_user_profile
    }

    pub(super) fn set_user_profile(&mut self, encrypted_user_profile: EncryptedUserProfile) {
        self.encrypted_user_profile = encrypted_user_profile;
    }
}

pub(crate) mod persistence {
    use phnxtypes::{
        codec::{BlobDecoded, BlobEncoded},
        identifiers::QualifiedUserName,
    };
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
            let record = query_scalar!(
                r#"SELECT
                    password_file AS "password_file: _"
                    encrypted_user_profile as "encrypted_user_profile: _"
                FROM as_user_records
                WHERE user_name = $1"#,
                user_name.to_string(),
            )
            .fetch_optional(connection)
            .await?;
            Ok(record.map(|record| {
                let BlobDecoded(password_file) = record.password_file;
                UserRecord::new(
                    user_name.clone(),
                    password_file,
                    record.encrypted_user_profile,
                )
            }))
        }

        /// Update the AsUserRecord for a given UserId.
        pub(crate) async fn update(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            let password_file = BlobEncoded(&self.password_file);
            query!(
                "UPDATE as_user_records SET password_file = $1, encrypted_user_profile = $2 WHERE user_name = $3",
                password_file as _,
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
            let password_file = BlobEncoded(&self.password_file);
            query!(
                "INSERT INTO as_user_records (user_name, password_file, encrypted_user_profile) VALUES ($1, $2, $3)",
                self.user_name.to_string(),
                password_file as _,
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
        use std::sync::LazyLock;

        use opaque_ke::{
            ClientRegistration, ClientRegistrationFinishParameters, ServerRegistration, ServerSetup,
        };
        use phnxtypes::{
            codec::PhnxCodec, crypto::OpaqueCiphersuite,
            messages::client_as_out::EncryptedUserProfile,
        };
        use rand::{CryptoRng, RngCore, SeedableRng, rngs::StdRng};
        use sqlx::PgPool;
        use uuid::Uuid;

        use super::*;

        pub(crate) fn generate_password_file(
            user_name: &QualifiedUserName,
            rng: &mut (impl CryptoRng + RngCore),
        ) -> anyhow::Result<ServerRegistration<OpaqueCiphersuite>> {
            let password = b"password";

            let server_setup = ServerSetup::new(rng);
            let client_registration_start_result =
                ClientRegistration::start(rng, password).unwrap();
            let server_registration_start_result = ServerRegistration::start(
                &server_setup,
                client_registration_start_result.message,
                user_name.to_string().as_bytes(),
            )
            .unwrap();
            let client_registration_finish_result = client_registration_start_result
                .state
                .finish(
                    rng,
                    password,
                    server_registration_start_result.message,
                    ClientRegistrationFinishParameters::default(),
                )
                .unwrap();

            Ok(ServerRegistration::finish(
                client_registration_finish_result.message,
            ))
        }

        pub(crate) async fn store_random_user_record(pool: &PgPool) -> anyhow::Result<UserRecord> {
            let user_name: QualifiedUserName = format!("{}@example.com", Uuid::new_v4()).parse()?;
            let password_file = generate_password_file(&user_name, &mut rand::thread_rng())?;
            let encrypted_user_profile = EncryptedUserProfile::dummy();
            let record = UserRecord {
                user_name,
                password_file,
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

        static PASSWORD_FILE: LazyLock<ServerRegistration<OpaqueCiphersuite>> =
            LazyLock::new(|| {
                let user_name: QualifiedUserName = "alice@example.com".parse().unwrap();
                generate_password_file(
                    &user_name,
                    &mut StdRng::seed_from_u64(0x0DDB1A5E5BAD5EEDu64),
                )
                .unwrap()
            });

        #[test]
        fn test_password_file_serde_codec() {
            insta::assert_binary_snapshot!(".cbor", PhnxCodec::to_vec(&*PASSWORD_FILE).unwrap());
        }

        #[test]
        fn test_password_file_serde_json() {
            insta::assert_json_snapshot!(&*PASSWORD_FILE);
        }
    }
}
