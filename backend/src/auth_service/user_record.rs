// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::ServerRegistration;
use phnxtypes::{crypto::OpaqueCiphersuite, identifiers::QualifiedUserName};

use crate::errors::StorageError;

#[derive(Debug, Clone)]
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

    pub(super) fn into_password_file(self) -> ServerRegistration<OpaqueCiphersuite> {
        self.password_file
    }
}

mod persistence {
    use phnxtypes::{
        codec::persist::{
            BlobPersist, BlobPersistRestore, BlobPersistStore, BlobPersisted, BlobPersisting,
        },
        identifiers::QualifiedUserName,
    };
    use serde::{Deserialize, Serialize};
    use sqlx::PgExecutor;

    use crate::errors::StorageError;

    use super::*;

    #[derive(Serialize)]
    struct StorableServerRegistrationRef<'a>(&'a ServerRegistration<OpaqueCiphersuite>);

    #[derive(Deserialize)]
    struct StoredServerRegistration(ServerRegistration<OpaqueCiphersuite>);

    impl BlobPersist for StoredServerRegistration {
        type Persisting<'a> = StorableServerRegistrationRef<'a>;
        type Persisted = Self;
    }

    impl BlobPersistStore for StorableServerRegistrationRef<'_> {}
    impl BlobPersistRestore for StoredServerRegistration {}

    impl UserRecord {
        /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
        /// exists for the given UserId.
        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
            user_name: &QualifiedUserName,
        ) -> Result<Option<UserRecord>, StorageError> {
            sqlx::query_scalar!(
                r#"SELECT password_file AS "password_file: _"
                FROM as_user_records
                WHERE user_name = $1"#,
                user_name.to_string(),
            )
            .fetch_optional(connection)
            .await?
            .map(|BlobPersisted(StoredServerRegistration(password_file))| {
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
            let password_file = StorableServerRegistrationRef(&self.password_file);
            sqlx::query!(
                "INSERT INTO as_user_records (user_name, password_file) VALUES ($1, $2)",
                self.user_name.to_string(),
                BlobPersisting(password_file) as _,
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
}
