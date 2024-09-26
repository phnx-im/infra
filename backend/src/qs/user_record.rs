// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::signatures::keys::QsUserVerifyingKey, identifiers::QsUserId, messages::FriendshipToken,
};
use sqlx::PgExecutor;

use crate::persistence::StorageError;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct UserRecord {
    pub(super) user_id: QsUserId,
    pub(super) verifying_key: QsUserVerifyingKey,
    pub(super) friendship_token: FriendshipToken,
}

impl UserRecord {
    pub(in crate::qs) async fn new_and_store(
        connection: impl PgExecutor<'_>,
        verifying_key: QsUserVerifyingKey,
        friendship_token: FriendshipToken,
    ) -> Result<Self, StorageError> {
        let user_id = QsUserId::random();
        let user_record = Self {
            user_id,
            verifying_key,
            friendship_token,
        };
        user_record.store(connection).await?;
        Ok(user_record)
    }
}

mod persistence {
    use phnxtypes::identifiers::QsUserId;
    use sqlx::PgExecutor;

    use crate::persistence::StorageError;

    use super::*;

    impl UserRecord {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO 
                    qs_user_records 
                    (user_id, verifying_key, friendship_token)
                VALUES 
                    ($1, $2, $3)",
                &self.user_id as &QsUserId,
                &self.verifying_key as &QsUserVerifyingKey,
                &self.friendship_token as &FriendshipToken,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
            user_id: &QsUserId,
        ) -> Result<Option<UserRecord>, StorageError> {
            sqlx::query!(
                r#"SELECT 
                    verifying_key as "verifying_key: QsUserVerifyingKey", friendship_token as "friendship_token: FriendshipToken"
                FROM 
                    qs_user_records
                WHERE 
                    user_id = $1"#,
                user_id.as_uuid(),
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                Ok(UserRecord {
                    user_id: user_id.clone(),
                    verifying_key: record.verifying_key,
                    friendship_token: record.friendship_token,
                })
            })
            .transpose()
        }

        pub(in crate::qs) async fn delete(
            connection: impl PgExecutor<'_>,
            user_id: QsUserId,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "DELETE FROM qs_user_records WHERE user_id = $1",
                &user_id as &QsUserId
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::qs) async fn update(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "UPDATE 
                    qs_user_records
                SET 
                    verifying_key = $2, friendship_token = $3
                WHERE 
                    user_id = $1",
                &self.user_id as &QsUserId,
                &self.verifying_key as &QsUserVerifyingKey,
                self.friendship_token.token(),
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }
}
