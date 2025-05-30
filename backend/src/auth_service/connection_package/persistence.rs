// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::{
    codec::{BlobDecoded, BlobEncoded},
    identifiers::{UserHandleHash, UserId},
    messages::client_as::ConnectionPackage,
};
use sqlx::{Arguments, PgConnection, PgExecutor, postgres::PgArguments};
use uuid::Uuid;

use crate::errors::StorageError;

use super::{StorableConnectionPackage, StorableConnectionPackageRef};

impl StorableConnectionPackage {
    pub(in crate::auth_service) async fn store_multiple(
        connection: impl PgExecutor<'_>,
        connection_packages: impl IntoIterator<Item = &ConnectionPackage>,
        user_id: &UserId,
    ) -> Result<(), StorageError> {
        let mut query_args = PgArguments::default();
        let mut query_string =
            String::from("INSERT INTO connection_packages (user_uuid, connection_package) VALUES");

        for (i, connection_package) in connection_packages.into_iter().enumerate() {
            let connection_package: StorableConnectionPackageRef = connection_package.into();

            // Add values to the query arguments. None of these should throw an error.
            query_args.add(user_id.uuid())?;
            query_args.add(BlobEncoded(connection_package))?;

            if i > 0 {
                query_string.push(',');
            }

            // Add placeholders for each value
            query_string.push_str(&format!(" (${}, ${})", i * 2 + 1, i * 2 + 2,));
        }

        // Finalize the query string
        query_string.push(';');

        // Execute the query
        sqlx::query_with(&query_string, query_args)
            .execute(connection)
            .await?;

        Ok(())
    }

    pub(in crate::auth_service) async fn store_multiple_for_handle(
        connection: impl PgExecutor<'_>,
        connection_packages: impl IntoIterator<Item = &ConnectionPackage>,
        hash: &UserHandleHash,
    ) -> Result<(), StorageError> {
        let mut query_args = PgArguments::default();
        let mut query_string = String::from(
            "INSERT INTO handle_connection_packages (hash, connection_package) VALUES",
        );

        for (i, connection_package) in connection_packages.into_iter().enumerate() {
            let connection_package: StorableConnectionPackageRef = connection_package.into();

            // Add values to the query arguments. None of these should throw an error.
            query_args.add(hash.as_bytes())?;
            query_args.add(BlobEncoded(connection_package))?;

            if i > 0 {
                query_string.push(',');
            }

            // Add placeholders for each value
            query_string.push_str(&format!(" (${}, ${})", i * 2 + 1, i * 2 + 2,));
        }

        // Finalize the query string
        query_string.push(';');

        // Execute the query
        sqlx::query_with(&query_string, query_args)
            .execute(connection)
            .await?;

        Ok(())
    }

    async fn load(connection: impl PgExecutor<'_>, user_uuid: Uuid) -> Result<Self, StorageError> {
        // This is to ensure that counting and deletion happen atomically. If we
        // don't do this, two concurrent queries might both count 2 and delete,
        // leaving us with 0 packages.
        sqlx::query_scalar!(
            r#"WITH next_connection_package AS (
                SELECT id, connection_package
                FROM connection_packages
                WHERE user_uuid = $1
                LIMIT 1
                FOR UPDATE -- make sure two concurrent queries don't return the same package
                SKIP LOCKED -- skip rows that are already locked by other processes
            ),
            remaining_packages AS (
                SELECT COUNT(*) as count
                FROM connection_packages
                WHERE user_uuid = $1
            ),
            deleted_package AS (
                DELETE FROM connection_packages
                WHERE id = (
                    SELECT id
                    FROM next_connection_package
                )
                AND (SELECT count FROM remaining_packages) > 1
            )
            SELECT connection_package
                AS "connection_package: BlobDecoded<StorableConnectionPackage>"
            FROM next_connection_package"#,
            user_uuid,
        )
        .fetch_one(connection)
        .await
        .map(|BlobDecoded(connection_package)| connection_package)
        .map_err(From::from)
    }

    pub(crate) async fn load_for_handle(
        connection: impl PgExecutor<'_>,
        hash: &UserHandleHash,
    ) -> sqlx::Result<ConnectionPackage> {
        // This is to ensure that counting and deletion happen atomically. If we
        // don't do this, two concurrent queries might both count 2 and delete,
        // leaving us with 0 packages.
        sqlx::query_scalar!(
            r#"WITH next_connection_package AS (
                SELECT id, connection_package
                FROM handle_connection_packages
                WHERE hash = $1
                LIMIT 1
                FOR UPDATE -- make sure two concurrent queries don't return the same package
                SKIP LOCKED -- skip rows that are already locked by other processes
            ),
            remaining_packages AS (
                SELECT COUNT(*) as count
                FROM handle_connection_packages
                WHERE hash = $1
            ),
            deleted_package AS (
                DELETE FROM handle_connection_packages
                WHERE id = (
                    SELECT id
                    FROM next_connection_package
                )
                AND (SELECT count FROM remaining_packages) > 1
            )
            SELECT connection_package
                AS "connection_package: BlobDecoded<StorableConnectionPackage>"
            FROM next_connection_package"#,
            hash.as_bytes(),
        )
        .fetch_one(connection)
        .await
        .map(|BlobDecoded(connection_package)| connection_package.into())
    }

    // TODO: Return only a single connection package
    /// Return a connection package for each client of a user referenced by a
    /// user name.
    pub(in crate::auth_service) async fn user_connection_packages(
        connection: &mut PgConnection,
        user_id: &UserId,
    ) -> Result<Vec<ConnectionPackage>, StorageError> {
        let connection_package = Self::load(connection, user_id.uuid()).await?;
        Ok(vec![connection_package.into()])
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use phnxcommon::{
        credentials::{ClientCredential, keys},
        crypto::{ConnectionDecryptionKey, signatures::signable::Signature},
        messages::{MlsInfraVersion, client_as::ConnectionPackageTbs},
        time::{Duration, ExpirationData},
    };
    use sqlx::PgPool;

    use crate::auth_service::{
        client_record::persistence::tests::{random_client_record, store_random_client_record},
        user_handles::UserHandleRecord,
        user_record::persistence::tests::store_random_user_record,
    };

    use super::*;

    async fn store_random_connection_packages(
        pool: &PgPool,
        user_id: &UserId,
        client_credential: ClientCredential,
    ) -> anyhow::Result<Vec<ConnectionPackage>> {
        let pkgs = vec![
            random_connection_package(client_credential.clone()),
            random_connection_package(client_credential),
        ];
        StorableConnectionPackage::store_multiple(pool, pkgs.iter(), user_id).await?;
        Ok(pkgs)
    }

    async fn store_random_connection_packages_for_handle(
        pool: &PgPool,
        hash: &UserHandleHash,
        client_credential: ClientCredential,
    ) -> anyhow::Result<Vec<ConnectionPackage>> {
        let pkgs = vec![
            random_connection_package(client_credential.clone()),
            random_connection_package(client_credential),
        ];
        StorableConnectionPackage::store_multiple_for_handle(pool, pkgs.iter(), hash).await?;
        Ok(pkgs)
    }

    pub(crate) fn random_connection_package(
        client_credential: ClientCredential,
    ) -> ConnectionPackage {
        ConnectionPackage::new_for_test(
            ConnectionPackageTbs::new(
                MlsInfraVersion::default(),
                ConnectionDecryptionKey::generate()
                    .unwrap()
                    .encryption_key()
                    .clone(),
                ExpirationData::new(Duration::days(90)),
                client_credential,
            ),
            Signature::new_for_test(b"signature".to_vec()),
        )
    }

    #[sqlx::test]
    async fn load(pool: PgPool) -> anyhow::Result<()> {
        let user_record = store_random_user_record(&pool).await?;
        let user_id = user_record.user_id().clone();
        let client_record = store_random_client_record(&pool, user_id.clone()).await?;
        let pkgs =
            store_random_connection_packages(&pool, &user_id, client_record.credential().clone())
                .await?;

        let mut loaded = [None, None];

        for _ in 0..2 {
            let pkg =
                StorableConnectionPackage::load(pool.acquire().await?.as_mut(), user_id.uuid())
                    .await?;
            let pkg: ConnectionPackage = pkg.into();
            if pkg == pkgs[0] {
                loaded[0] = Some(pkg);
            } else if pkg == pkgs[1] {
                loaded[1] = Some(pkg);
            }
        }

        assert_eq!(loaded[0].as_ref(), Some(&pkgs[0]));
        assert_eq!(loaded[1].as_ref(), Some(&pkgs[1]));

        Ok(())
    }

    #[sqlx::test]
    async fn user_connection_packages(pool: PgPool) -> anyhow::Result<()> {
        let user_record = store_random_user_record(&pool).await?;

        let user_id = user_record.user_id().clone();
        let client_record_a = store_random_client_record(&pool, user_id.clone()).await?;
        let pkgs =
            store_random_connection_packages(&pool, &user_id, client_record_a.credential().clone())
                .await?;

        let loaded = StorableConnectionPackage::user_connection_packages(
            pool.acquire().await?.as_mut(),
            user_record.user_id(),
        )
        .await?;

        assert_eq!(loaded.len(), 1);
        assert!(loaded[0] == pkgs[0] || loaded[0] == pkgs[1]);

        Ok(())
    }

    #[sqlx::test]
    async fn handle_connection_packages(pool: PgPool) -> anyhow::Result<()> {
        let hash = UserHandleHash::new([1; 32]);
        UserHandleRecord {
            user_handle_hash: hash,
            verifying_key: keys::HandleVerifyingKey::from_bytes(vec![1, 2, 3, 4, 5]),
            expiration_data: ExpirationData::new(Duration::days(1)),
        }
        .store(&pool)
        .await?;

        let user_id = UserId::random("example.com".parse()?);
        let client_record = random_client_record(user_id.clone())?;

        let mut pkgs = store_random_connection_packages_for_handle(
            &pool,
            &hash,
            client_record.credential().clone(),
        )
        .await?;

        let loaded =
            StorableConnectionPackage::load_for_handle(pool.acquire().await?.as_mut(), &hash)
                .await?;

        // first or second package is loaded
        assert!(loaded == pkgs[0] || loaded == pkgs[1]);
        if loaded == pkgs[0] {
            pkgs.remove(0);
        } else {
            pkgs.remove(1);
        }

        // remaing package is loaded
        let loaded =
            StorableConnectionPackage::load_for_handle(pool.acquire().await?.as_mut(), &hash)
                .await?;
        assert!(loaded == pkgs[0]);

        // last package is not deleted
        let loaded =
            StorableConnectionPackage::load_for_handle(pool.acquire().await?.as_mut(), &hash)
                .await?;
        assert!(loaded == pkgs[0]);

        Ok(())
    }
}
