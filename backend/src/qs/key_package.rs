// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::messages::QsEncryptedKeyPackage;

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
    use sqlx::{Arguments, Connection, PgConnection, PgExecutor, postgres::PgArguments};

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
            connection: impl PgExecutor<'_>,
            user_id: &QsUserId,
            client_id: &QsClientId,
        ) -> Result<Option<Self>, StorageError> {
            sqlx::query_scalar!(
                r#"WITH to_delete AS (
                    SELECT id FROM key_packages
                    INNER JOIN qs_client_records qcr
                        ON qcr.client_id = key_packages.client_id
                    WHERE
                        key_packages.client_id = $1
                        AND qcr.user_id = $2
                    LIMIT 1
                    FOR UPDATE SKIP LOCKED
                )
                DELETE FROM key_packages
                WHERE id IN (SELECT id FROM to_delete)
                RETURNING encrypted_key_package AS "eap: _"
                "#,
                client_id as &QsClientId,
                user_id as &QsUserId
            )
            .fetch_optional(connection)
            .await
            .map_err(From::from)
        }

        pub(in crate::qs) async fn load_user_key_package(
            connection: &mut PgConnection,
            friendship_token: &FriendshipToken,
        ) -> Result<Self, StorageError> {
            let mut transaction = connection.begin().await?;

            let encrypted_key_package = sqlx::query_scalar!(
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
            ).fetch_one(&mut *transaction).await?;

            transaction.commit().await?;

            Ok(encrypted_key_package)
        }
    }

    #[cfg(test)]
    mod tests {

        use sqlx::PgPool;

        use crate::qs::{
            client_record::persistence::tests::store_random_client_record,
            user_record::persistence::tests::store_random_user_record,
        };

        use super::*;

        #[sqlx::test]
        async fn store_multiple_load(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(&pool, user_record.user_id).await?;
            let packages = store_random_key_packages(&pool, &client_record.client_id).await?;

            let mut loaded = [None, None];

            for _ in 0..2 {
                let pkg = StorableEncryptedAddPackage::load(
                    &pool,
                    &user_record.user_id,
                    &client_record.client_id,
                )
                .await?
                .expect("missing key package");

                if pkg.0 == packages[0] {
                    loaded[0] = Some(pkg);
                } else if pkg.0 == packages[1] {
                    loaded[1] = Some(pkg);
                }
            }

            let pkg = StorableEncryptedAddPackage::load(
                &pool,
                &user_record.user_id,
                &client_record.client_id,
            )
            .await?;
            assert!(pkg.is_none());

            assert_eq!(loaded[0].as_ref().unwrap().0, packages[0]);
            assert_eq!(loaded[1].as_ref().unwrap().0, packages[1]);

            Ok(())
        }

        #[sqlx::test]
        async fn load_user_key_package(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(&pool, user_record.user_id).await?;
            let packages = store_random_key_packages(&pool, &client_record.client_id).await?;

            let mut loaded = [None, None];

            for _ in 0..2 {
                let pkg = StorableEncryptedAddPackage::load_user_key_package(
                    pool.acquire().await?.as_mut(),
                    &user_record.friendship_token,
                )
                .await?;
                if pkg.0 == packages[0] {
                    loaded[0] = Some(pkg);
                } else if pkg.0 == packages[1] {
                    loaded[1] = Some(pkg);
                }
            }

            assert_eq!(loaded[0].as_ref().unwrap().0, packages[0]);
            assert_eq!(loaded[1].as_ref().unwrap().0, packages[1]);

            Ok(())
        }

        async fn store_random_key_packages(
            pool: &PgPool,
            client_id: &QsClientId,
        ) -> anyhow::Result<Vec<QsEncryptedKeyPackage>> {
            let pkg_a = QsEncryptedKeyPackage::random();
            let pkg_b = QsEncryptedKeyPackage::random();
            StorableEncryptedAddPackage::store_multiple(pool, client_id, [&pkg_a, &pkg_b]).await?;
            Ok(vec![pkg_a, pkg_b])
        }
    }
}
