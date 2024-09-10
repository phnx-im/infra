// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::ServerRegistration;
use phnxtypes::{crypto::OpaqueCiphersuite, identifiers::UserName};

#[derive(Debug, Clone)]
struct UserRecord {
    _user_name: UserName,
    password_file: ServerRegistration<OpaqueCiphersuite>,
}

impl UserRecord {
    fn new(user_name: UserName, password_file: ServerRegistration<OpaqueCiphersuite>) -> Self {
        Self {
            _user_name: user_name,
            password_file,
        }
    }
}

mod persistence {
    use opaque_ke::ServerRegistration;
    use phnxtypes::{codec::PhnxCodec, crypto::OpaqueCiphersuite, identifiers::UserName};
    use sqlx::PgExecutor;
    use uuid::Uuid;

    use crate::persistence::StorageError;

    use super::UserRecord;

    impl UserRecord {
        /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
        /// exists for the given UserId.
        async fn load(
            connection: impl PgExecutor<'_>,
            user_name: &UserName,
        ) -> Result<Option<UserRecord>, StorageError> {
            sqlx::query!(
                "SELECT user_name, password_file FROM as_user_records WHERE user_name = $1",
                user_name,
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                let password_file = PhnxCodec::from_slice(&record.password_file)?;
                Ok(UserRecord::new(user_name.clone(), password_file))
            })
            .transpose()
        }
    }

    /// Create a new user with the given user name. If a user with the given user
    /// name already exists, an error is returned.
    async fn new_and_store(
        connection: impl PgExecutor<'_>,
        user_name: &UserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
    ) -> Result<(), StorageError> {
        let id = Uuid::new_v4();
        let user_name_bytes = PhnxCodec::to_vec(user_name)?;
        let password_file_bytes = PhnxCodec::to_vec(&opaque_record)?;
        sqlx::query!(
            "INSERT INTO as_user_records (id, user_name, password_file) VALUES ($1, $2, $3)",
            id,
            user_name_bytes,
            password_file_bytes,
        )
        .execute(&self.pool)
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
    async fn delete_user(&self, user_id: &UserName) -> Result<(), Self::DeleteUserError> {
        let user_name_bytes = PhnxCodec::to_vec(user_id)?;
        // The database cascades the delete to the clients and their connection packages.
        sqlx::query!(
            "DELETE FROM as_user_records WHERE user_name = $1",
            user_name_bytes
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
