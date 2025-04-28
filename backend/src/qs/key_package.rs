// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackage;

use phnxtypes::{
    codec::{BlobDecoded, BlobEncoded},
    identifiers::QsClientId,
    messages::FriendshipToken,
};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::{Arguments, Connection, PgConnection, PgExecutor, postgres::PgArguments};
use tonic::async_trait;

use crate::errors::StorageError;

impl StorableKeyPackage for KeyPackage {}

#[async_trait]
pub(super) trait StorableKeyPackage:
    Sized + Serialize + DeserializeOwned + Clone + Send + Sync + Unpin
{
    async fn store_multiple(
        connection: impl PgExecutor<'_>,
        client_id: &QsClientId,
        key_packages: &[Self],
    ) -> Result<(), StorageError> {
        Self::store_multiple_internal(connection, client_id, key_packages, false).await
    }

    async fn store_last_resort(
        &self,
        connection: impl PgExecutor<'_>,
        client_id: &QsClientId,
    ) -> Result<(), StorageError> {
        Self::store_multiple_internal(connection, client_id, &[self.clone()], true).await
    }

    async fn store_multiple_internal(
        connection: impl PgExecutor<'_>,
        client_id: &QsClientId,
        key_packages: &[Self],
        is_last_resort: bool,
    ) -> Result<(), StorageError> {
        let mut query_args = PgArguments::default();
        let mut query_string = String::from(
            "INSERT INTO key_packages (client_id, encrypted_key_package, is_last_resort) VALUES",
        );

        for (i, key_package) in key_packages.into_iter().enumerate() {
            // Add values to the query arguments. None of these should throw an error.
            query_args.add(client_id)?;
            query_args.add(BlobEncoded(key_package))?;
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

    async fn load_user_key_package(
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
                SELECT encrypted_key_package as "key_package: BlobDecoded<Self>" FROM selected_key_packages"#,
                friendship_token as &FriendshipToken
            ).fetch_one(&mut *transaction).await.map(|BlobDecoded(key_package)| key_package)?;

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

    type DummyKeyPackage = Vec<u8>;

    impl StorableKeyPackage for DummyKeyPackage {}

    #[sqlx::test]
    async fn load_user_key_package(pool: PgPool) -> anyhow::Result<()> {
        let user_record = store_random_user_record(&pool).await?;
        let client_record = store_random_client_record(&pool, user_record.user_id).await?;
        let packages = store_random_key_packages(&pool, &client_record.client_id).await?;

        let mut loaded = [None, None];

        for _ in 0..2 {
            let pkg = DummyKeyPackage::load_user_key_package(
                pool.acquire().await?.as_mut(),
                &user_record.friendship_token,
            )
            .await?;
            if pkg == packages[0] {
                loaded[0] = Some(pkg);
            } else if pkg == packages[1] {
                loaded[1] = Some(pkg);
            }
        }

        assert_eq!(loaded[0].as_ref().unwrap(), &packages[0]);
        assert_eq!(loaded[1].as_ref().unwrap(), &packages[1]);

        Ok(())
    }

    async fn store_random_key_packages(
        pool: &PgPool,
        client_id: &QsClientId,
    ) -> anyhow::Result<Vec<DummyKeyPackage>> {
        let pkg_a = vec![1, 2, 3, 4];
        let pkg_b = vec![5, 6, 7, 8];
        DummyKeyPackage::store_multiple(pool, client_id, &[pkg_a.clone(), pkg_b.clone()]).await?;
        Ok(vec![pkg_a, pkg_b])
    }
}
