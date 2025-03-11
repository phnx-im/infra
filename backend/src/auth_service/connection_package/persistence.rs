// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    identifiers::{AsClientId, QualifiedUserName},
    messages::client_as::ConnectionPackage,
};
use sqlx::{postgres::PgArguments, Arguments, Connection, PgConnection, PgExecutor};
use uuid::Uuid;

use crate::errors::StorageError;

use super::{StorableConnectionPackage, StorableConnectionPackageRef};

impl StorableConnectionPackage {
    // TODO: No need to take items by value
    pub(in crate::auth_service) async fn store_multiple(
        connection: impl PgExecutor<'_>,
        connection_packages: impl IntoIterator<Item = &ConnectionPackage>,
        client_id: &AsClientId,
    ) -> Result<(), StorageError> {
        let mut query_args = PgArguments::default();
        let mut query_string =
            String::from("INSERT INTO connection_packages (client_id, connection_package) VALUES");

        for (i, connection_package) in connection_packages.into_iter().enumerate() {
            let connection_package: StorableConnectionPackageRef = connection_package.into();

            // Add values to the query arguments. None of these should throw an error.
            query_args.add(client_id.client_id())?;
            query_args.add(connection_package)?;

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

    async fn load(connection: impl PgExecutor<'_>, client_id: Uuid) -> Result<Self, StorageError> {
        // This is to ensure that counting and deletion happen atomically. If we
        // don't do this, two concurrent queries might both count 2 and delete,
        // leaving us with 0 packages.
        sqlx::query_scalar!(
            r#"WITH next_connection_package AS (
                SELECT id, connection_package
                FROM connection_packages
                WHERE client_id = $1
                LIMIT 1
                FOR UPDATE -- make sure two concurrent queries don't return the same package
                SKIP LOCKED -- skip rows that are already locked by other processes
            ),
            remaining_packages AS (
                SELECT COUNT(*) as count
                FROM connection_packages
                WHERE client_id = $1
            ),
            deleted_package AS (
                DELETE FROM connection_packages
                WHERE id = (
                    SELECT id
                    FROM next_connection_package
                )
                AND (SELECT count FROM remaining_packages) > 1
            )
            SELECT connection_package AS "connection_package: StorableConnectionPackage"
            FROM next_connection_package"#,
            client_id,
        )
        .fetch_one(connection)
        .await
        .map_err(From::from)
    }

    /// TODO: Last resort key package
    pub(in crate::auth_service) async fn client_connection_package(
        connection: &mut PgConnection,
        client_id: &AsClientId,
    ) -> Result<ConnectionPackage, StorageError> {
        Self::load(connection, client_id.client_id())
            .await
            .map(From::from)
    }

    /// Return a connection package for each client of a user referenced by a
    /// user name.
    pub(in crate::auth_service) async fn user_connection_packages(
        connection: &mut PgConnection,
        user_name: &QualifiedUserName,
    ) -> Result<Vec<ConnectionPackage>, StorageError> {
        // Start the transaction
        let mut transaction = connection.begin().await?;

        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *transaction)
            .await?;

        // Collect all client ids associated with that user.
        let client_ids = sqlx::query_scalar!(
            "SELECT client_id FROM as_client_records WHERE user_name = $1",
            user_name.to_string(),
        )
        .fetch_all(&mut *transaction)
        .await?;

        // First fetch all connection package records from the DB.
        let mut connection_packages = Vec::with_capacity(client_ids.len());
        for client_id in client_ids {
            let connection_package = Self::load(&mut *transaction, client_id).await?;
            connection_packages.push(connection_package.into());
        }

        // End the transaction.
        transaction.commit().await?;

        Ok(connection_packages)
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::{
        credentials::ClientCredential,
        crypto::{signatures::signable::Signature, ConnectionDecryptionKey},
        messages::{client_as::ConnectionPackageTbs, MlsInfraVersion},
        time::{Duration, ExpirationData},
    };
    use sqlx::PgPool;

    use crate::auth_service::{
        client_record::persistence::tests::store_random_client_record,
        user_record::persistence::tests::store_random_user_record,
    };

    use super::*;

    async fn store_random_connection_packages(
        pool: &PgPool,
        client_id: &AsClientId,
        client_credential: ClientCredential,
    ) -> anyhow::Result<Vec<ConnectionPackage>> {
        let pkgs = vec![
            random_connection_package(client_credential.clone()),
            random_connection_package(client_credential),
        ];
        StorableConnectionPackage::store_multiple(pool, pkgs.iter(), client_id).await?;
        Ok(pkgs)
    }

    fn random_connection_package(client_credential: ClientCredential) -> ConnectionPackage {
        ConnectionPackage::new_for_test(
            ConnectionPackageTbs::new(
                MlsInfraVersion::default(),
                ConnectionDecryptionKey::generate()
                    .unwrap()
                    .encryption_key(),
                ExpirationData::new(Duration::days(90)),
                client_credential,
            ),
            Signature::new_for_test(b"signature".to_vec()),
        )
    }

    #[sqlx::test]
    async fn load(pool: PgPool) -> anyhow::Result<()> {
        let user_record = store_random_user_record(&pool).await?;
        let client_id = AsClientId::new(user_record.user_name().clone(), Uuid::new_v4());
        let client_record = store_random_client_record(&pool, client_id.clone()).await?;
        let pkgs =
            store_random_connection_packages(&pool, &client_id, client_record.credential().clone())
                .await?;

        let mut loaded = [None, None];

        for _ in 0..2 {
            let pkg = StorableConnectionPackage::load(
                pool.acquire().await?.as_mut(),
                client_id.client_id(),
            )
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

        let client_id = AsClientId::new(user_record.user_name().clone(), Uuid::new_v4());
        let client_record_a = store_random_client_record(&pool, client_id.clone()).await?;
        let pkgs_a = store_random_connection_packages(
            &pool,
            &client_id,
            client_record_a.credential().clone(),
        )
        .await?;

        let client_id = AsClientId::new(user_record.user_name().clone(), Uuid::new_v4());
        let client_record_b = store_random_client_record(&pool, client_id.clone()).await?;
        let pkgs_b = store_random_connection_packages(
            &pool,
            &client_id,
            client_record_b.credential().clone(),
        )
        .await?;

        let loaded = StorableConnectionPackage::user_connection_packages(
            pool.acquire().await?.as_mut(),
            user_record.user_name(),
        )
        .await?;

        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains(&pkgs_a[0]) || loaded.contains(&pkgs_a[1]));
        assert!(loaded.contains(&pkgs_b[0]) || loaded.contains(&pkgs_b[1]));

        Ok(())
    }
}
