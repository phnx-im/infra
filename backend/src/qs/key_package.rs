// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::keypackage_batch::QsEncryptedKeyPackage;

#[derive(sqlx::Type)]
#[sqlx(transparent)]
pub(super) struct StorableEncryptedAddPackage(pub QsEncryptedKeyPackage);

impl Deref for StorableEncryptedAddPackage {
    type Target = QsEncryptedKeyPackage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

mod persistence {
    use phnxtypes::{
        identifiers::{QsClientId, QsUserId},
        messages::FriendshipToken,
    };
    use sqlx::{postgres::PgArguments, Arguments, Connection, PgConnection, PgExecutor};

    use crate::errors::StorageError;

    use super::*;
    impl StorableEncryptedAddPackage {
        pub(in crate::qs) async fn store_multiple(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
            encrypted_key_packages: impl IntoIterator<Item = impl Deref<Target = QsEncryptedKeyPackage>>,
        ) -> Result<(), StorageError> {
            Self::store_multiple_internal(connection, client_id, encrypted_key_packages, false)
                .await
        }

        pub(in crate::qs) async fn store_last_resort(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
            encrypted_key_package: impl Deref<Target = QsEncryptedKeyPackage>,
        ) -> Result<(), StorageError> {
            Self::store_multiple_internal(connection, client_id, [encrypted_key_package], true)
                .await
        }

        async fn store_multiple_internal(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
            encrypted_key_packages: impl IntoIterator<Item = impl Deref<Target = QsEncryptedKeyPackage>>,
            is_last_resort: bool,
        ) -> Result<(), StorageError> {
            let mut query_args = PgArguments::default();
            let mut query_string = String::from(
                "INSERT INTO key_packages (client_id, encrypted_key_package, is_last_resort) VALUES",
            );

            for (i, encrypted_key_package) in encrypted_key_packages.into_iter().enumerate() {
                // Add values to the query arguments. None of these should throw an error.
                query_args.add(client_id)?;
                query_args.add(&*encrypted_key_package)?;
                query_args.add(is_last_resort)?;

                if i > 0 {
                    query_string.push(',');
                }

                // Add placeholders for each value
                query_string.push_str(&format!(
                    " (${}, ${}, ${})",
                    i * 3 + 1,
                    i * 3 + 2,
                    i * 3 + 3,
                ));
            }

            // Finalize the query string
            query_string.push(';');

            // Execute the query
            sqlx::query_with(&query_string, query_args)
                .execute(connection)
                .await?;

            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: &mut PgConnection,
            user_id: &QsUserId,
            client_id: &QsClientId,
        ) -> Result<Option<Self>, StorageError> {
            let mut transaction = connection.begin().await?;

            let encrypted_key_package_option = sqlx::query_scalar!(
                r#"WITH deleted_package AS (
                    DELETE FROM key_packages
                    USING qs_client_records qcr
                    WHERE
                        key_packages.client_id = qcr.client_id
                        AND key_packages.client_id = $1
                        AND qcr.user_id = $2
                    RETURNING key_packages.id, key_packages.encrypted_key_package
                )
                SELECT encrypted_key_package as "eap: _" FROM deleted_package
                FOR UPDATE SKIP LOCKED"#,
                client_id as &QsClientId,
                user_id as &QsUserId
            )
            .fetch_optional(&mut *transaction)
            .await?;

            transaction.commit().await?;

            Ok(encrypted_key_package_option)
        }

        pub(in crate::qs) async fn load_user_key_packages(
            connection: &mut PgConnection,
            friendship_token: &FriendshipToken,
        ) -> Result<Vec<Self>, StorageError> {
            let mut transaction = connection.begin().await?;

            let encrypted_key_packages = sqlx::query_scalar!(
                r#"WITH user_info AS (
                    -- Step 1: Fetch the user_id based on the friendship token.
                    SELECT user_id FROM qs_user_records WHERE friendship_token = $1
                ),

                client_ids AS (
                    -- Step 2: Retrieve client IDs for the user from the `user_info`.
                    SELECT client_id FROM qs_client_records WHERE user_id = (SELECT user_id FROM user_info)
                ),

                ranked_packages AS (
                    -- Step 3: Rank key packages for each client.
                    SELECT p.id, p.encrypted_key_package, p.is_last_resort,
                           ROW_NUMBER() OVER (PARTITION BY p.client_id ORDER BY p.is_last_resort ASC) AS rn
                    FROM key_packages p
                    INNER JOIN client_ids c ON p.client_id = c.client_id
                ),

                selected_key_packages AS (
                    -- Step 4: Select the best-ranked package per client (rn = 1), skipping locked rows.
                    SELECT id, encrypted_key_package, is_last_resort
                    FROM ranked_packages
                    WHERE rn = 1
                    FOR UPDATE SKIP LOCKED
                ),

                deleted_packages AS (
                    -- Step 5: Delete the selected packages that are not marked as last_resort.
                    DELETE FROM key_packages
                    WHERE id IN (SELECT id FROM selected_key_packages WHERE is_last_resort = FALSE)
                    RETURNING encrypted_key_package
                )

                -- Step 6: Return the encrypted_key_package from the selected packages.
                SELECT encrypted_key_package as "eap: _" FROM selected_key_packages"#,
                friendship_token as &FriendshipToken
            ).fetch_all(&mut *transaction).await?;

            transaction.commit().await?;

            Ok(encrypted_key_packages)
        }
    }
}
