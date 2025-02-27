// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    codec::persist::{BlobPersist, BlobPersisted},
    identifiers::{AsClientId, QualifiedUserName},
    messages::client_as::ConnectionPackage,
};
use sqlx::{postgres::PgArguments, Arguments, Connection, PgConnection, PgExecutor};
use uuid::Uuid;

use crate::errors::StorageError;

use super::StorableConnectionPackage;

impl StorableConnectionPackage {
    pub(in crate::auth_service) async fn store_multiple(
        connection: impl PgExecutor<'_>,
        connection_packages: impl IntoIterator<Item = impl Into<StorableConnectionPackage>>,
        client_id: &AsClientId,
    ) -> Result<(), StorageError> {
        let mut query_args = PgArguments::default();
        let mut query_string =
            String::from("INSERT INTO connection_packages (client_id, connection_package) VALUES");

        for (i, connection_package) in connection_packages.into_iter().enumerate() {
            let connection_package: StorableConnectionPackage = connection_package.into();

            // Add values to the query arguments. None of these should throw an error.
            query_args.add(client_id.client_id())?;
            query_args.add(connection_package.persisting())?;

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

    async fn load(
        connection: &mut PgConnection,
        client_id: Uuid,
    ) -> Result<StorableConnectionPackage, StorageError> {
        let mut transaction = connection.begin().await?;

        // This is to ensure that counting and deletion happen atomically. If we
        // don't do this, two concurrent queries might both count 2 and delete,
        // leaving us with 0 packages.
        let BlobPersisted(connection_package) = sqlx::query_scalar!(
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
                RETURNING connection_package
            )
            SELECT connection_package AS "connection_package: _"
            FROM next_connection_package"#,
            client_id,
        )
        .fetch_one(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(connection_package)
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
        let client_ids_record = sqlx::query!(
            "SELECT client_id FROM as_client_records WHERE user_name = $1",
            user_name.to_string(),
        )
        .fetch_all(&mut *transaction)
        .await?;

        // First fetch all connection package records from the DB.
        let mut connection_packages: Vec<ConnectionPackage> =
            Vec::with_capacity(client_ids_record.len());
        for client_id in client_ids_record {
            let connection_package = Self::load(&mut transaction, client_id.client_id).await?;
            connection_packages.push(connection_package.into());
        }

        // End the transaction.
        transaction.commit().await?;

        Ok(connection_packages)
    }
}
