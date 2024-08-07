// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::configurations::DatabaseSettings;
use async_trait::async_trait;
use mls_assist::openmls_traits::types::SignatureScheme;
use num_traits::ToPrimitive;
use opaque_ke::{rand::rngs::OsRng, ServerRegistration, ServerSetup};
use phnxbackend::auth_service::{
    storage_provider_trait::AsStorageProvider, AsClientRecord, AsUserRecord,
};
use phnxtypes::{
    credentials::{
        keys::{AsIntermediateSigningKey, AsSigningKey},
        AsCredential, AsIntermediateCredential, AsIntermediateCredentialCsr, ClientCredential,
        CredentialFingerprint,
    },
    crypto::OpaqueCiphersuite,
    identifiers::{AsClientId, Fqdn, UserName},
    messages::{client_as::ConnectionPackage, QueueMessage},
    time::TimeStamp,
};
use privacypass::{
    batched_tokens_ristretto255::{server::BatchedKeyStore, Ristretto255, VoprfServer},
    TruncatedTokenKeyId,
};
#[cfg(feature = "sqlite_provider")]
use rusqlite::{types::FromSql, ToSql};
use sqlx::{
    postgres::PgArguments,
    types::{BigDecimal, Uuid},
    Acquire, Arguments, PgConnection, PgPool, Row,
};
use thiserror::Error;

use super::connect_to_database;

pub struct PostgresAsStorage {
    pool: PgPool,
}

impl PostgresAsStorage {
    pub async fn new(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
        settings: &DatabaseSettings,
    ) -> Result<Self, CreateAsStorageError> {
        let pool = connect_to_database(settings).await?;

        let provider = Self { pool };

        // Check if the database has been initialized.
        let (as_creds, _as_inter_creds, _) = provider.load_as_credentials().await?;
        if as_creds.is_empty() {
            let (as_signing_key, as_inter_signing_key) =
                generate_fresh_credentials(as_domain, signature_scheme)?;
            let _ = sqlx::query!(
                r#"INSERT INTO as_signing_keys (id, cred_type, credential_fingerprint, signing_key, currently_active) VALUES ($1, $2, $3, $4, $5)"#,
                Uuid::new_v4(),
                CredentialType::As as _,
                as_signing_key.credential().fingerprint().as_bytes(),
                serde_json::to_vec(&as_signing_key)?,
                true,
            )
            .execute(&provider.pool)
            .await?;
            let _ = sqlx::query!(
                r#"INSERT INTO as_signing_keys (id, cred_type, credential_fingerprint, signing_key, currently_active) VALUES ($1, $2, $3, $4, $5)"#,
                Uuid::new_v4(),
                CredentialType::Intermediate as _,
                as_inter_signing_key.credential().fingerprint().as_bytes(),
                serde_json::to_vec(&as_inter_signing_key)?,
                true,
            )
            .execute(&provider.pool)
            .await?;
        }
        if provider.load_opaque_setup().await.is_err() {
            let mut rng = OsRng;
            let opaque_setup = ServerSetup::<OpaqueCiphersuite>::new(&mut rng);
            let _ = sqlx::query!(
                r#"INSERT INTO opaque_setup (id, opaque_setup) VALUES ($1, $2)"#,
                Uuid::new_v4(),
                serde_json::to_vec(&opaque_setup)?,
            )
            .execute(&provider.pool)
            .await?;
        };
        Ok(provider)
    }

    async fn load_connection_package_internal(
        transaction: &mut PgConnection,
        client_id: Uuid,
    ) -> Result<Vec<u8>, AsPostgresError> {
        let mut savepoint = transaction.begin().await?;

        // TODO: Set the isolation level to SERIALIZABLE. This is necessary
        // because we're counting the number of packages and then deleting one.
        // We should do this once we're moving to a proper state-machine model
        // for server storage and networking.

        // This is to ensure that counting and deletion happen atomically. If we
        // don't do this, two concurrent queries might both count 2 and delete,
        // leaving us with 0 packages.
        //sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        //    .execute(&mut *savepoint)
        //    .await?;

        let connection_package_bytes_record = sqlx::query!(
            "WITH next_connection_package AS (
                SELECT id, connection_package 
                FROM connection_packages 
                WHERE client_id = $1 LIMIT 1
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
        .fetch_one(&mut *savepoint)
        .await?;

        savepoint.commit().await?;

        Ok(connection_package_bytes_record.connection_package)
    }
}

pub(crate) fn generate_fresh_credentials(
    as_domain: Fqdn,
    signature_scheme: SignatureScheme,
) -> Result<(AsSigningKey, AsIntermediateSigningKey), CreateAsStorageError> {
    let (_credential, as_signing_key) =
        AsCredential::new(signature_scheme, as_domain.clone(), None)
            .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
    let (csr, prelim_signing_key) = AsIntermediateCredentialCsr::new(signature_scheme, as_domain)
        .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
    let as_intermediate_credential = csr
        .sign(&as_signing_key, None)
        .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
    let as_intermediate_signing_key =
        AsIntermediateSigningKey::from_prelim_key(prelim_signing_key, as_intermediate_credential)
            .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
    Ok((as_signing_key, as_intermediate_signing_key))
}

#[async_trait]
impl BatchedKeyStore for PostgresAsStorage {
    /// Inserts a keypair with a given `token_key_id` into the key store.
    async fn insert(&self, token_key_id: TruncatedTokenKeyId, server: VoprfServer<Ristretto255>) {
        let Ok(server_bytes) = serde_json::to_vec(&server) else {
            return;
        };
        let _ = sqlx::query!(
            "INSERT INTO as_batched_keys (token_key_id, voprf_server) VALUES ($1, $2)",
            token_key_id as i16,
            server_bytes,
        )
        .execute(&self.pool)
        .await;
    }
    /// Returns a keypair with a given `token_key_id` from the key store.
    async fn get(&self, token_key_id: &TruncatedTokenKeyId) -> Option<VoprfServer<Ristretto255>> {
        let server_bytes_record = sqlx::query!(
            "SELECT voprf_server FROM as_batched_keys WHERE token_key_id = $1",
            *token_key_id as i16,
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;
        let server = serde_json::from_slice(&server_bytes_record.voprf_server).ok()?;
        Some(server)
    }
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "credential_type", rename_all = "lowercase")]
pub(crate) enum CredentialType {
    As,
    Intermediate,
}

#[cfg(feature = "sqlite_provider")]
impl FromSql for CredentialType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = i16::column_result(value)?;
        match value {
            0 => Ok(CredentialType::As),
            1 => Ok(CredentialType::Intermediate),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

#[cfg(feature = "sqlite_provider")]
impl ToSql for CredentialType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            CredentialType::As => 0.to_sql(),
            CredentialType::Intermediate => 1.to_sql(),
        }
    }
}

#[async_trait]
impl AsStorageProvider for PostgresAsStorage {
    type PrivacyPassKeyStore = Self;
    type StorageError = AsPostgresError;

    type CreateUserError = AsPostgresError;
    type StoreUserError = AsPostgresError;
    type DeleteUserError = AsPostgresError;

    type StoreClientError = AsPostgresError;
    type CreateClientError = AsPostgresError;
    type DeleteClientError = AsPostgresError;

    type EnqueueError = QueueError;
    type ReadAndDeleteError = QueueError;

    type StoreKeyPackagesError = AsPostgresError;
    type LoadConnectionPackageError = AsPostgresError;

    type LoadSigningKeyError = AsPostgresError;
    type LoadAsCredentialsError = AsPostgresError;

    type LoadOpaqueKeyError = AsPostgresError;

    // === Users ===

    /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
    /// exists for the given UserId.
    async fn load_user(&self, user_name: &UserName) -> Option<AsUserRecord> {
        let user_name_bytes = serde_json::to_vec(user_name).ok()?;
        let user_record = sqlx::query!(
            "SELECT user_name, password_file FROM as_user_records WHERE user_name = $1",
            user_name_bytes,
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;
        let password_file = serde_json::from_slice(&user_record.password_file).ok()?;
        let as_user_record = AsUserRecord::new(user_name.clone(), password_file);
        Some(as_user_record)
    }

    /// Create a new user with the given user name. If a user with the given user
    /// name already exists, an error is returned.
    async fn create_user(
        &self,
        user_name: &UserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError> {
        let id = Uuid::new_v4();
        let user_name_bytes = serde_json::to_vec(user_name)?;
        let password_file_bytes = serde_json::to_vec(&opaque_record)?;
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
        let user_name_bytes = serde_json::to_vec(user_id)?;
        // The database cascades the delete to the clients and their connection packages.
        sqlx::query!(
            "DELETE FROM as_user_records WHERE user_name = $1",
            user_name_bytes
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // === Clients ===

    async fn create_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::CreateClientError> {
        let user_name_bytes = serde_json::to_vec(&client_id.user_name())?;
        let queue_encryption_key_bytes = serde_json::to_vec(&client_record.queue_encryption_key)?;
        let ratchet = serde_json::to_vec(&client_record.ratchet_key)?;
        let activity_time = client_record.activity_time.time();
        let client_credential = serde_json::to_vec(&client_record.credential)?;
        sqlx::query!(
            "INSERT INTO as_client_records (client_id, user_name, queue_encryption_key, ratchet, activity_time, client_credential, remaining_tokens) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            client_id.client_id(),
            user_name_bytes,
            queue_encryption_key_bytes,
            ratchet,
            activity_time,
            client_credential,
            1000, // TODO: Once we use tokens, we should make this configurable.
        )
        .execute(&self.pool)
        .await?;
        // Initialize the client's queue.
        let initial_sequence_number = BigDecimal::from(0u8);

        sqlx::query!(
            "INSERT INTO as_queue_data (queue_id, sequence_number) VALUES ($1, $2)",
            client_id.client_id(),
            initial_sequence_number
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Load the info for the client with the given client ID.
    async fn load_client(&self, client_id: &AsClientId) -> Option<AsClientRecord> {
        let user_record = sqlx::query!(
            "SELECT * FROM as_client_records WHERE client_id = $1",
            client_id.client_id(),
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;
        let queue_encryption_key =
            serde_json::from_slice(&user_record.queue_encryption_key).ok()?;
        let ratchet_key = serde_json::from_slice(&user_record.ratchet).ok()?;
        let activity_time = TimeStamp::from(user_record.activity_time);
        let credential = serde_json::from_slice(&user_record.client_credential).ok()?;
        let as_client_record =
            AsClientRecord::new(queue_encryption_key, ratchet_key, activity_time, credential);
        Some(as_client_record)
    }

    /// Saves a client in the storage provider with the given client ID. The
    /// storage provider must associate this client with the user of the client.
    async fn store_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        let user_name_bytes = serde_json::to_vec(&client_id.user_name())?;
        let queue_encryption_key_bytes = serde_json::to_vec(&client_record.queue_encryption_key)?;
        let ratchet = serde_json::to_vec(&client_record.ratchet_key)?;
        let activity_time = client_record.activity_time.time();
        let client_credential = serde_json::to_vec(&client_record.credential)?;
        sqlx::query!(
            "UPDATE as_client_records SET user_name = $2, queue_encryption_key = $3, ratchet = $4, activity_time = $5, client_credential = $6, remaining_tokens = $7 WHERE client_id = $1",
            client_id.client_id(),
            user_name_bytes,
            queue_encryption_key_bytes,
            ratchet,
            activity_time,
            client_credential,
            1000, // TODO: Once we use tokens, we should make this configurable.
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes the client with the given client ID.
    ///
    /// The storage provider must also delete the following:
    ///  - The associated user, if the user has no other clients
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_client(&self, client_id: &AsClientId) -> Result<(), Self::StorageError> {
        sqlx::query!(
            "DELETE FROM as_client_records WHERE client_id = $1",
            client_id.client_id(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // === Key packages ===

    /// Store connection packages for a specific client.
    async fn store_connection_packages(
        &self,
        client_id: &AsClientId,
        connection_packages: Vec<ConnectionPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        let mut query_args = PgArguments::default();
        let mut query_string = String::from(
            "INSERT INTO connection_packages (id, client_id, connection_package) VALUES",
        );

        for (i, connection_package) in connection_packages.iter().enumerate() {
            let id = Uuid::new_v4();
            let connection_package_bytes = serde_json::to_vec(&connection_package)?;

            // Add values to the query arguments
            query_args.add(id);
            query_args.add(client_id.client_id());
            query_args.add(connection_package_bytes);

            if i > 0 {
                query_string.push(',');
            }

            // Add placeholders for each value
            query_string.push_str(&format!(
                " (${}, ${}, ${})",
                i * 3 + 1,
                i * 3 + 2,
                i * 3 + 3
            ));
        }

        // Finalize the query string
        query_string.push(';');

        // Execute the query
        sqlx::query_with(&query_string, query_args)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Return a key package for a specific client. The client_id must belong to
    /// the same user as the requested key packages.
    /// TODO: Last resort key package
    async fn client_connection_package(
        &self,
        client_id: &AsClientId,
    ) -> Result<ConnectionPackage, Self::LoadConnectionPackageError> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;

        let connection_package_bytes =
            Self::load_connection_package_internal(&mut tx, client_id.client_id()).await?;

        tx.commit().await?;

        let connection_package = serde_json::from_slice(&connection_package_bytes)?;

        Ok(connection_package)
    }

    /// Return a connection package for each client of a user referenced by a
    /// user name.
    async fn load_user_connection_packages(
        &self,
        user_name: &UserName,
    ) -> Result<Vec<ConnectionPackage>, Self::StorageError> {
        let user_name_bytes = serde_json::to_vec(user_name)?;

        // Start the transaction
        let mut transaction = self.pool.begin().await?;

        // Collect all client ids associated with that user.
        let client_ids_record = sqlx::query!(
            "SELECT client_id FROM as_client_records WHERE user_name = $1",
            user_name_bytes
        )
        .fetch_all(&mut *transaction)
        .await?;

        // First fetch all connection package records from the DB.
        let mut connection_packages_bytes = Vec::new();
        for client_id in client_ids_record {
            let connection_package_bytes =
                Self::load_connection_package_internal(&mut transaction, client_id.client_id)
                    .await?;
            connection_packages_bytes.push(connection_package_bytes);
        }

        // End the transaction.
        transaction.commit().await?;

        // Deserialize the connection packages.
        let connection_packages = connection_packages_bytes
            .into_iter()
            .map(|connection_package_bytes| serde_json::from_slice(&connection_package_bytes))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(connection_packages)
    }

    // === Messages ===

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    async fn enqueue(
        &self,
        client_id: &AsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError> {
        // Encode the message
        let message_bytes = serde_json::to_vec(&message)?;

        // Begin the transaction
        let mut transaction = self.pool.begin().await?;

        // Check if sequence numbers are consistent.
        let sequence_number_record = sqlx::query!(
            "SELECT sequence_number FROM as_queue_data WHERE queue_id = $1 FOR UPDATE",
            client_id.client_id(),
        )
        .fetch_one(&mut *transaction)
        .await?;

        // We're storing things as the NUMERIC postgres type. We need the
        // num-traits crate to convert to u64. If we find a better way to store
        // u64s, we might be able to get rid of that dependency.
        let sequence_number_decimal: BigDecimal = sequence_number_record.sequence_number;
        let sequence_number = sequence_number_decimal
            .to_u64()
            // The conversion should be successful, as we're only writing u64s
            // to the DB in the first place.
            .ok_or_else(|| QueueError::LibraryError)?;

        if sequence_number != message.sequence_number {
            tracing::warn!(
                "Sequence number mismatch. Message sequence number {}, queue sequence number {}",
                message.sequence_number,
                sequence_number
            );
            return Err(QueueError::SequenceNumberMismatch);
        }

        // Get a fresh message ID (only used as a unique key for postgres)
        let message_id = Uuid::new_v4();
        // Store the message in the DB
        sqlx::query!(
            "INSERT INTO as_queues (message_id, queue_id, sequence_number, message_bytes) VALUES ($1, $2, $3, $4)",
            message_id,
            client_id.client_id(),
            sequence_number_decimal,
            message_bytes,
        )
        .execute(&mut *transaction)
        .await?;

        let new_sequence_number = sequence_number_decimal + BigDecimal::from(1u8);
        // Increase the sequence number and store it.
        sqlx::query!(
            "UPDATE as_queue_data SET sequence_number = $2 WHERE queue_id = $1",
            client_id.client_id(),
            new_sequence_number
        )
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }

    /// Delete all messages older than the given sequence number in the queue
    /// with the given client ID and return up to the requested number of
    /// messages from the queue starting with the message with the given
    /// sequence number, as well as the number of unread messages remaining in
    /// the queue.
    async fn read_and_delete(
        &self,
        client_id: &AsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError> {
        let sequence_number_decimal = BigDecimal::from(sequence_number);
        // TODO: sqlx wants an i64 here and in a few other places below, but
        // we're using u64s. This is probably a limitation of postgres and we
        // might want to change some of the input/output types accordingly.
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| QueueError::LibraryError)?;

        let mut transaction = self.pool.begin().await?;

        // This query is idempotent, so there's no need to lock anything.
        let query = "WITH deleted AS (
                DELETE FROM as_queues 
                WHERE queue_id = $1 AND sequence_number < $2
            ),
            fetched AS (
                SELECT message_bytes FROM as_queues
                WHERE queue_id = $1 AND sequence_number >= $2
                ORDER BY sequence_number ASC
                LIMIT $3
            ),
            remaining AS (
                SELECT COUNT(*) AS count 
                FROM as_queues
                WHERE queue_id = $1 AND sequence_number >= $2
            )
            SELECT 
                fetched.message_bytes,
                remaining.count
            FROM fetched, remaining";

        let rows = sqlx::query(query)
            .bind(client_id.client_id())
            .bind(sequence_number_decimal)
            .bind(number_of_messages)
            .fetch_all(&mut *transaction)
            .await?;

        transaction.commit().await?;

        // Convert the records to messages.
        let messages = rows
            .iter()
            .map(|row| {
                let message_bytes: &[u8] = row.try_get("message_bytes")?;
                let message = serde_json::from_slice(message_bytes)?;
                Ok(message)
            })
            .collect::<Result<Vec<_>, QueueError>>()?;

        let remaining_messages = if let Some(row) = rows.first() {
            let remaining_count: i64 = row.try_get("count")?;
            // Subtract the number of messages we've read from the remaining
            // count to get the number of unread messages.
            remaining_count - messages.len() as i64
        } else {
            0
        };

        return Ok((messages, remaining_messages as u64));
    }

    /// Load the currently active signing key and the
    /// [`AsIntermediateCredential`].
    async fn load_signing_key(
        &self,
    ) -> Result<AsIntermediateSigningKey, Self::LoadSigningKeyError> {
        let signing_key_bytes_record = sqlx::query!("SELECT signing_key FROM as_signing_keys WHERE currently_active = true AND cred_type = 'intermediate'")
            .fetch_one(&self.pool)
            .await?;
        let signing_key = serde_json::from_slice(&signing_key_bytes_record.signing_key)?;
        Ok(signing_key)
    }

    /// Load all currently active [`AsCredential`]s and
    /// [`AsIntermediateCredential`]s.
    async fn load_as_credentials(
        &self,
    ) -> Result<
        (
            Vec<AsCredential>,
            Vec<AsIntermediateCredential>,
            Vec<CredentialFingerprint>,
        ),
        Self::LoadAsCredentialsError,
    > {
        // TODO: The postgres provider currently does not yet support revoked credentials.
        let revoked_fingerprints = vec![];
        let signing_keys_bytes_record = sqlx::query!(
            r#"SELECT signing_key, cred_type AS "cred_type: CredentialType" FROM as_signing_keys WHERE currently_active = true"#
        )
        .fetch_all(&self.pool)
        .await?;
        let mut intermed_creds = vec![];
        let mut as_creds = vec![];
        for record in signing_keys_bytes_record {
            match record.cred_type {
                CredentialType::As => {
                    let as_cred: AsSigningKey = serde_json::from_slice(&record.signing_key)?;
                    as_creds.push(as_cred.credential().clone());
                }
                CredentialType::Intermediate => {
                    let intermed_cred: AsIntermediateSigningKey =
                        serde_json::from_slice(&record.signing_key)?;
                    intermed_creds.push(intermed_cred.credential().clone());
                }
            }
        }
        Ok((as_creds, intermed_creds, revoked_fingerprints))
    }

    /// Load the OPAQUE [`ServerSetup`].
    async fn load_opaque_setup(
        &self,
    ) -> Result<ServerSetup<OpaqueCiphersuite>, Self::LoadSigningKeyError> {
        // There is currently only one OPAQUE setup.
        let opaque_setup_record = sqlx::query!("SELECT opaque_setup FROM opaque_setup")
            .fetch_one(&self.pool)
            .await?;
        let opaque_setup = serde_json::from_slice(&opaque_setup_record.opaque_setup)?;
        Ok(opaque_setup)
    }

    // === Anonymous requests ===

    /// Return the client credentials of a user for a given username.
    async fn client_credentials(&self, user_name: &UserName) -> Vec<ClientCredential> {
        let Ok(user_name_bytes) = serde_json::to_vec(user_name) else {
            return vec![];
        };
        let Ok(client_records) = sqlx::query!(
            "SELECT client_credential FROM as_client_records WHERE user_name = $1",
            user_name_bytes,
        )
        .fetch_all(&self.pool)
        .await
        else {
            return vec![];
        };
        let mut client_credentials = Vec::new();
        for client_record in client_records {
            let Ok(client_credential) = serde_json::from_slice(&client_record.client_credential)
            else {
                continue;
            };
            client_credentials.push(client_credential);
        }
        client_credentials
    }

    // === PrivacyPass ===

    /// Loads the handle of the PrivacyPass keystore.
    async fn privacy_pass_key_store(&self) -> &Self::PrivacyPassKeyStore {
        self
    }

    /// Loads the number of tokens is still allowed to request.
    async fn load_client_token_allowance(
        &self,
        client_id: &AsClientId,
    ) -> Result<usize, Self::StorageError> {
        let remaining_tokens_record = sqlx::query!(
            "SELECT remaining_tokens FROM as_client_records WHERE client_id = $1",
            client_id.client_id(),
        )
        .fetch_one(&self.pool)
        .await?;
        let remaining_tokens = remaining_tokens_record.remaining_tokens;
        // TODO: Unsafe conversion.
        Ok(remaining_tokens as usize)
    }

    async fn set_client_token_allowance(
        &self,
        client_id: &AsClientId,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError> {
        sqlx::query!(
            "UPDATE as_client_records SET remaining_tokens = $2 WHERE client_id = $1",
            client_id.client_id(),
            // TODO: Unsafe conversion.
            number_of_tokens as i16,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Resets the token allowance of all clients. This should be called after a
    /// rotation of the privacy pass token issuance key material.
    async fn reset_token_allowances(
        &self,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError> {
        sqlx::query!(
            "UPDATE as_client_records SET remaining_tokens = $1",
            // TODO: Unsafe conversion.
            number_of_tokens as i16,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum CreateAsStorageError {
    #[error(transparent)]
    ProviderError(#[from] AsPostgresError),
    #[error(transparent)]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    #[error(transparent)]
    CodecError(#[from] serde_json::Error),
    /// Credential generation error.
    #[error("Credential generation error.")]
    CredentialGenerationError,
}

#[derive(Debug, Error)]
pub enum AsPostgresError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    #[error(transparent)]
    CodecError(#[from] serde_json::Error),
}

/// General error while accessing the requested queue.
#[derive(Error, Debug)]
pub enum QueueError {
    #[error(transparent)]
    PostgresError(#[from] sqlx::Error),
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Error serializing message
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
}
