// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    codec::PhnxCodec,
    identifiers::{AsClientId, QualifiedUserName},
    messages::client_as::ConnectionPackage,
};
use sqlx::{postgres::PgArguments, Arguments, Connection, PgConnection, PgExecutor};
use uuid::Uuid;

use crate::persistence::StorageError;

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
            let connection_package_bytes = PhnxCodec::to_vec(&connection_package)?;

            // Add values to the query arguments. None of these should throw an error.
            query_args.add(client_id.client_id())?;
            query_args.add(connection_package_bytes)?;

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

    async fn load(connection: &mut PgConnection, client_id: Uuid) -> Result<Vec<u8>, StorageError> {
        let mut transaction = connection.begin().await?;

        // This is to ensure that counting and deletion happen atomically. If we
        // don't do this, two concurrent queries might both count 2 and delete,
        // leaving us with 0 packages.
        let connection_package_bytes_record = sqlx::query!(
            "WITH next_connection_package AS (
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
            SELECT id, connection_package FROM next_connection_package",
            client_id,
        )
        .fetch_one(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(connection_package_bytes_record.connection_package)
    }

    /// TODO: Last resort key package
    pub(in crate::auth_service) async fn client_connection_package(
        connection: &mut PgConnection,
        client_id: &AsClientId,
    ) -> Result<ConnectionPackage, StorageError> {
        let connection_package_bytes = Self::load(connection, client_id.client_id()).await?;

        let connection_package: StorableConnectionPackage =
            PhnxCodec::from_slice(&connection_package_bytes)?;

        Ok(connection_package.into())
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
        let mut connection_packages_bytes = Vec::new();
        for client_id in client_ids_record {
            let connection_package_bytes =
                Self::load(&mut transaction, client_id.client_id).await?;
            connection_packages_bytes.push(connection_package_bytes);
        }

        // End the transaction.
        transaction.commit().await?;

        // Deserialize the connection packages.
        let connection_packages = connection_packages_bytes
            .into_iter()
            .map(|connection_package_bytes| {
                let storable: StorableConnectionPackage =
                    PhnxCodec::from_slice(&connection_package_bytes)?;
                Ok(storable.into())
            })
            .collect::<Result<Vec<ConnectionPackage>, StorageError>>()?;

        Ok(connection_packages)
    }
}
