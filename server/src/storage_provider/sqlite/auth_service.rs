// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mls_assist::openmls_traits::types::SignatureScheme;
use opaque_ke::{rand::rngs::OsRng, ServerRegistration, ServerSetup};
use phnxbackend::auth_service::{
    storage_provider_trait::AsStorageProvider, AsClientRecord, AsUserRecord,
};
use phnxtypes::{
    credentials::{
        keys::{AsIntermediateSigningKey, AsSigningKey},
        AsCredential, AsIntermediateCredential, ClientCredential, CredentialFingerprint,
    },
    crypto::OpaqueCiphersuite,
    identifiers::{AsClientId, Fqdn, UserName},
    messages::{client_as::ConnectionPackage, QueueMessage},
    time::TimeStamp,
};
use privacypass::{
    batched_tokens_ristretto255::server::BatchedKeyStore,
    private_tokens::{Ristretto255, VoprfServer},
    TruncatedTokenKeyId,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use sqlx::types::Uuid;
use thiserror::Error;

use crate::storage_provider::postgres::auth_service::{generate_fresh_credentials, CredentialType};

#[derive(Debug, Error)]
pub enum AsSqliteError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    CodecError(#[from] phnxtypes::codec::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Credential generation error.
    #[error("Credential generation error.")]
    CredentialGenerationError,
}

/// General error while accessing the requested queue.
#[derive(Error, Debug)]
pub enum QueueError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Error serializing message
    #[error(transparent)]
    SerializationError(#[from] phnxtypes::codec::Error),
    #[error("Mutex poisoned")]
    MutexError,
}

pub struct SqliteAsStorage {
    connection: SqliteConnection,
}

impl SqliteAsStorage {
    pub async fn new(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
        db_path: &str,
    ) -> Result<Self, AsSqliteError> {
        let connection = Connection::open(db_path)?;
        Self::initialize_db(as_domain, signature_scheme, connection).await
    }

    pub async fn new_in_memory(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
    ) -> Result<Self, AsSqliteError> {
        let connection = Connection::open_in_memory()?;
        Self::initialize_db(as_domain, signature_scheme, connection).await
    }

    async fn initialize_db(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
        connection: Connection,
    ) -> Result<Self, AsSqliteError> {
        let connection = Arc::new(Mutex::new(connection));
        let provider = Self { connection };
        provider.create_tables()?;

        // Generate fresh credentials
        let (as_creds, _as_inter_creds, _) = provider.load_as_credentials().await?;
        if as_creds.is_empty() {
            tracing::info!("Generating fresh AS credentials.");
            let connection = provider
                .connection
                .lock()
                .map_err(|_| AsSqliteError::MutexError)?;
            let (as_signing_key, as_inter_signing_key) =
                generate_fresh_credentials(as_domain, signature_scheme)
                    .map_err(|_| AsSqliteError::CredentialGenerationError)?;
            connection.execute(
                "INSERT INTO as_signing_keys (id, cred_type, credential_fingerprint, signing_key, currently_active) VALUES ($1, $2, $3, $4, $5)",
                params![
                    Uuid::new_v4(),
                    CredentialType::As,
                    as_signing_key.credential().fingerprint().as_bytes(),
                    phnxtypes::codec::to_vec(&as_signing_key)?,
                    true,
                ],
            )?;
            connection.execute(
                "INSERT INTO as_signing_keys (id, cred_type, credential_fingerprint, signing_key, currently_active) VALUES ($1, $2, $3, $4, $5)",
                params![
                    Uuid::new_v4(),
                    CredentialType::Intermediate,
                    as_inter_signing_key.credential().fingerprint().as_bytes(),
                    phnxtypes::codec::to_vec(&as_inter_signing_key)?,
                    true,
                ],
            )?;
            drop(connection);
        }

        let res = provider.load_as_credentials().await;
        debug_assert!(res.is_ok(), "Failed to load AS credentials.");

        if provider.load_opaque_setup().await.is_err() {
            let mut rng = OsRng;
            let opaque_setup = ServerSetup::<OpaqueCiphersuite>::new(&mut rng);
            let connection = provider
                .connection
                .lock()
                .map_err(|_| AsSqliteError::MutexError)?;
            connection.execute(
                "INSERT INTO opaque_setup (id, opaque_setup) VALUES ($1, $2)",
                params![Uuid::new_v4(), phnxtypes::codec::to_vec(&opaque_setup)?],
            )?;
        };

        Ok(provider)
    }

    fn create_tables(&self) -> Result<(), AsSqliteError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS as_batched_keys(
                    token_key_id INTEGER PRIMARY KEY,
                    voprf_server BLOB NOT NULL
                )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS as_user_records (
                id UUID PRIMARY KEY,
                user_name BLOB NOT NULL,
                password_file BLOB NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS as_signing_keys (
                id UUID PRIMARY KEY,
                signing_key BLOB NOT NULL,
                currently_active BOOLEAN NOT NULL,
                credential_fingerprint BLOB NOT NULL,
                cred_type INTEGER NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS opaque_setup(
                id UUID PRIMARY KEY,
                opaque_setup BLOB NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS as_client_records (
                client_id UUID PRIMARY KEY,
                user_name BLOB NOT NULL,
                queue_encryption_key BLOB NOT NULL,
                ratchet BLOB NOT NULL,
                activity_time INTEGER NOT NULL,
                client_credential BLOB NOT NULL,
                remaining_tokens INTEGER NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS connection_packages (
                id UUID PRIMARY KEY,
                client_id UUID NOT NULL,
                connection_package BLOB NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS queues (
                message_id UUID PRIMARY KEY,
                queue_id UUID NOT NULL,
                sequence_number INTEGER NOT NULL,
                message_bytes BLOB NOT NULL
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS queue_data (
                queue_id UUID PRIMARY KEY,
                sequence_number INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }
}

#[async_trait]
impl BatchedKeyStore for SqliteAsStorage {
    /// Inserts a keypair with a given `token_key_id` into the key store.
    async fn insert(&self, token_key_id: TruncatedTokenKeyId, server: VoprfServer<Ristretto255>) {
        let Ok(server_bytes) = phnxtypes::codec::to_vec(&server) else {
            return;
        };

        let Ok(connection) = self.connection.lock() else {
            return;
        };

        let _ = connection.execute(
            "INSERT INTO as_batched_keys (token_key_id, voprf_server) VALUES ($1, $2)",
            params![token_key_id as i16, &server_bytes],
        );
    }
    /// Returns a keypair with a given `token_key_id` from the key store.
    async fn get(&self, token_key_id: &TruncatedTokenKeyId) -> Option<VoprfServer<Ristretto255>> {
        let Ok(connection) = self.connection.lock() else {
            return None;
        };

        let server_bytes_record: Vec<u8> = connection
            .query_row(
                "SELECT voprf_server FROM as_batched_keys WHERE token_key_id = $1",
                params![*token_key_id as i16],
                |row| row.get(0),
            )
            .optional()
            .ok()??;

        let server = phnxtypes::codec::from_slice(&server_bytes_record).ok()?;
        Some(server)
    }
}

#[async_trait]
impl AsStorageProvider for SqliteAsStorage {
    type PrivacyPassKeyStore = Self;
    type StorageError = AsSqliteError;

    type CreateUserError = AsSqliteError;
    type StoreUserError = AsSqliteError;
    type DeleteUserError = AsSqliteError;

    type StoreClientError = AsSqliteError;
    type CreateClientError = AsSqliteError;
    type DeleteClientError = AsSqliteError;

    type EnqueueError = QueueError;
    type ReadAndDeleteError = QueueError;

    type StoreKeyPackagesError = AsSqliteError;

    type LoadSigningKeyError = AsSqliteError;
    type LoadAsCredentialsError = AsSqliteError;

    type LoadOpaqueKeyError = AsSqliteError;

    // === Users ===

    /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
    /// exists for the given UserId.
    async fn load_user(&self, user_name: &UserName) -> Option<AsUserRecord> {
        let user_name_bytes = phnxtypes::codec::to_vec(user_name).ok()?;

        let connection = self.connection.lock().ok()?;

        connection
            .query_row(
                "SELECT user_name, password_file FROM as_user_records WHERE user_name = $1",
                params![user_name_bytes],
                |row| {
                    let user_name_bytes: Vec<u8> = row.get(0)?;
                    let user_name: UserName =
                        phnxtypes::codec::from_slice(&user_name_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                        })?;
                    let password_file_bytes: Vec<u8> = row.get(1)?;

                    let password_file: ServerRegistration<OpaqueCiphersuite> =
                        phnxtypes::codec::from_slice(&password_file_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, Box::new(e))
                        })?;

                    Ok(AsUserRecord::new(user_name, password_file))
                },
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Create a new user with the given user name. If a user with the given user
    /// name already exists, an error is returned.
    async fn create_user(
        &self,
        user_name: &UserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError> {
        let id = Uuid::new_v4();
        let user_name_bytes = phnxtypes::codec::to_vec(user_name)?;
        let password_file_bytes = phnxtypes::codec::to_vec(&opaque_record)?;

        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        connection.execute(
            "INSERT INTO as_user_records (id, user_name, password_file) VALUES ($1, $2, $3)",
            params![id, user_name_bytes, password_file_bytes],
        )?;

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
        let user_name_bytes = phnxtypes::codec::to_vec(user_id)?;

        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        connection.execute(
            "DELETE FROM as_user_records WHERE user_name = $1",
            params![user_name_bytes],
        )?;

        // Delete the relevant client data.
        todo!()
    }

    // === Clients ===

    async fn create_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::CreateClientError> {
        let user_name_bytes = phnxtypes::codec::to_vec(&client_id.user_name())?;
        let queue_encryption_key_bytes = phnxtypes::codec::to_vec(&client_record.queue_encryption_key)?;
        let ratchet = phnxtypes::codec::to_vec(&client_record.ratchet_key)?;
        let activity_time = client_record.activity_time.time();
        let client_credential = phnxtypes::codec::to_vec(&client_record.credential)?;

        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        connection.execute(
            "INSERT INTO as_client_records (client_id, user_name, queue_encryption_key, ratchet, activity_time, client_credential, remaining_tokens) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            params![
                client_id.client_id(),
                user_name_bytes,
                queue_encryption_key_bytes,
                ratchet,
                activity_time,
                client_credential,
                1000, // TODO: Once we use tokens, we should make this configurable.
            ],
        )?;

        // Initialize the client's queue.
        let initial_sequence_number = 0u8;

        connection.execute(
            "INSERT INTO queue_data (queue_id, sequence_number) VALUES ($1, $2)",
            params![client_id.client_id(), initial_sequence_number],
        )?;

        Ok(())
    }

    /// Load the info for the client with the given client ID.
    async fn load_client(&self, client_id: &AsClientId) -> Option<AsClientRecord> {
        let connection = self.connection.lock().ok()?;

        let res = connection
            .query_row(
                "SELECT * FROM as_client_records WHERE client_id = $1",
                params![client_id.client_id()],
                |row| {
                    let queue_encryption_key_bytes: Vec<u8> = row.get(2)?;
                    let queue_encryption_key = phnxtypes::codec::from_slice(&queue_encryption_key_bytes)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(2, Type::Blob, Box::new(e))
                        })?;

                    let ratchet: Vec<u8> = row.get(3)?;
                    let ratchet_key = phnxtypes::codec::from_slice(&ratchet).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(3, Type::Blob, Box::new(e))
                    })?;
                    let activity_time: DateTime<Utc> = row.get(4)?;
                    let activity_time = TimeStamp::from(activity_time);

                    let client_credential_bytes: Vec<u8> = row.get(5)?;
                    let credential: ClientCredential =
                        phnxtypes::codec::from_slice(&client_credential_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(5, Type::Blob, Box::new(e))
                        })?;

                    Ok(AsClientRecord::new(
                        queue_encryption_key,
                        ratchet_key,
                        activity_time,
                        credential,
                    ))
                },
            )
            .optional();
        match res {
            Ok(client_record) => client_record,
            Err(e) => {
                tracing::error!("Error loading client record: {:?}", e);
                None
            }
        }
    }

    /// Saves a client in the storage provider with the given client ID. The
    /// storage provider must associate this client with the user of the client.
    async fn store_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        let user_name_bytes = phnxtypes::codec::to_vec(&client_id.user_name())?;
        let queue_encryption_key_bytes = phnxtypes::codec::to_vec(&client_record.queue_encryption_key)?;
        let ratchet = phnxtypes::codec::to_vec(&client_record.ratchet_key)?;
        let activity_time = client_record.activity_time.time();
        let client_credential = phnxtypes::codec::to_vec(&client_record.credential)?;

        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        connection.execute(
            "UPDATE as_client_records SET user_name = $2, queue_encryption_key = $3, ratchet = $4, activity_time = $5, client_credential = $6, remaining_tokens = $7 WHERE client_id = $1",
            params![
                client_id.client_id(),
                user_name_bytes,
                queue_encryption_key_bytes,
                ratchet,
                activity_time,
                client_credential,
                1000, // TODO: Once we use tokens, we should make this configurable.
            ],
        )?;

        Ok(())
    }

    /// Deletes the client with the given client ID.
    ///
    /// The storage provider must also delete the following:
    ///  - The associated user, if the user has no other clients
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_client(&self, client_id: &AsClientId) -> Result<(), Self::StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;
        connection.execute(
            "DELETE FROM as_client_records WHERE client_id = $1",
            params![client_id.client_id()],
        )?;

        Ok(())
    }

    // === Key packages ===

    /// Store connection packages for a specific client.
    async fn store_connection_packages(
        &self,
        client_id: &AsClientId,
        connection_packages: Vec<ConnectionPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;
        // TODO: This can probably be improved. For now, we insert each connection
        // package individually.
        for connection_package in connection_packages {
            let id = Uuid::new_v4();
            let connection_package_bytes = phnxtypes::codec::to_vec(&connection_package)?;
            connection.execute(
                "INSERT INTO connection_packages (id, client_id, connection_package) VALUES ($1, $2, $3)",
                params![id, client_id.client_id(), connection_package_bytes],
            )?;
        }
        Ok(())
    }

    /// Return a key package for a specific client. The client_id must belong to
    /// the same user as the requested key packages.
    /// TODO: Last resort key package
    async fn client_connection_package(&self, client_id: &AsClientId) -> Option<ConnectionPackage> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)
            .ok()?;

        let (id, connection_package) = connection
            .query_row(
                "SELECT id, connection_package FROM connection_packages WHERE client_id = $1",
                params![client_id.client_id()],
                |row| {
                    let id: Uuid = row.get(0)?;
                    let connection_package_bytes: Vec<u8> = row.get(1)?;
                    let connection_package: ConnectionPackage =
                        phnxtypes::codec::from_slice(&connection_package_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, Box::new(e))
                        })?;
                    Ok((id, connection_package))
                },
            )
            .ok()?;
        let remaining_add_packages: i64 = connection
            .query_row(
                "SELECT COUNT(*) as count FROM connection_packages WHERE client_id = $1",
                params![client_id.client_id()],
                |row| row.get(0),
            )
            .ok()?;

        if remaining_add_packages > 1 {
            connection
                .execute("DELETE FROM connection_packages WHERE id = $1", params![id])
                .ok()?;
        };

        Some(connection_package)
    }

    /// Return a connection package for each client of a user referenced by a
    /// user name.
    async fn load_user_connection_packages(
        &self,
        user_name: &UserName,
    ) -> Result<Vec<ConnectionPackage>, Self::StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        let user_name_bytes = phnxtypes::codec::to_vec(user_name)?;

        // Collect all client ids associated with that user.
        let client_ids = connection
            .prepare("SELECT client_id FROM as_client_records WHERE user_name = ?")?
            .query_map(params![user_name_bytes], |row| row.get(0))?
            .collect::<Result<Vec<Uuid>, rusqlite::Error>>()?;

        let mut connection_packages = Vec::new();
        for client_id in client_ids {
            let (id, connection_package) = connection.query_row(
                "SELECT id, connection_package FROM connection_packages WHERE client_id = $1",
                params![client_id],
                |row| {
                    let id: Uuid = row.get(0)?;
                    let connection_package_bytes: Vec<u8> = row.get(1)?;
                    let connection_package: ConnectionPackage =
                        phnxtypes::codec::from_slice(&connection_package_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, Box::new(e))
                        })?;
                    Ok((id, connection_package))
                },
            )?;
            connection.execute("DELETE FROM connection_packages WHERE id = $1", params![id])?;

            connection_packages.push(connection_package);
        }
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
        let connection = self.connection.lock().map_err(|_| QueueError::MutexError)?;

        // Check if sequence numbers are consistent.
        let sequence_number: u64 = connection.query_row(
            "SELECT sequence_number FROM queue_data WHERE queue_id = ?",
            params![client_id.client_id()],
            |row| row.get(0),
        )?;

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
        let message_bytes = phnxtypes::codec::to_vec(&message)?;

        // Store the message in the DB
        connection.execute(
            "INSERT INTO queues (message_id, queue_id, sequence_number, message_bytes) VALUES (?, ?, ?, ?)",
            params![message_id, client_id.client_id(), message.sequence_number, message_bytes],
        )?;

        let new_sequence_number = sequence_number + 1;
        // Increase the sequence number and store it.
        connection.execute(
            "UPDATE queue_data SET sequence_number = ? WHERE queue_id = ?",
            params![new_sequence_number, client_id.client_id()],
        )?;

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
        // TODO: We can probably combine these three queries into one.

        // Delete all messages until the given "last seen" one.
        let connection = self.connection.lock().map_err(|_| QueueError::MutexError)?;

        connection.execute(
            "DELETE FROM queues WHERE queue_id = ? AND sequence_number < ?",
            params![client_id.client_id(), sequence_number],
        )?;

        // Now fetch at most `number_of_messages` messages from the queue.

        // TODO: sqlx wants an i64 here and in a few other places below, but
        // we're using u64s. This is probably a limitation of postgres and we
        // might want to change some of the input/output types accordingly.
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| QueueError::LibraryError)?;

        let messages = connection
            .prepare(
                "SELECT message_bytes FROM queues WHERE queue_id = ? ORDER BY sequence_number ASC LIMIT ?",
            )?
            .query_map(params![client_id.client_id(), number_of_messages], |row| {
                let message_bytes: Vec<u8> = row.get(0)?;
                let message: QueueMessage = phnxtypes::codec::from_slice(&message_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                        })?;
                Ok(message)
            })?
            .collect::<Result<Vec<QueueMessage>, rusqlite::Error>>()?;

        let lower_limit = sequence_number + messages.len() as u64;
        let remaining_messages: i64 = connection.query_row(
            "SELECT COUNT(*) FROM queues WHERE queue_id = ? AND sequence_number >= ?",
            params![client_id.client_id(), lower_limit],
            |row| row.get(0),
        )?;

        return Ok((messages, remaining_messages as u64));
    }

    /// Load the currently active signing key and the
    /// [`AsIntermediateCredential`].
    async fn load_signing_key(
        &self,
    ) -> Result<AsIntermediateSigningKey, Self::LoadSigningKeyError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        let signing_key = connection
            .query_row(
                "SELECT signing_key FROM as_signing_keys WHERE currently_active = true AND cred_type = ?",
                [CredentialType::Intermediate],
                |row| {
                    let signing_key_bytes: Vec<u8> = row.get(0)?;
                    let signing_key: AsIntermediateSigningKey =
                        phnxtypes::codec::from_slice(&signing_key_bytes).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                        })?;
                    Ok(signing_key)
                },
            )?;

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
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        // TODO: The sqlite provider currently does not yet support revoked credentials.
        let revoked_fingerprints = vec![];
        let signing_key_bytes: Vec<(Vec<u8>, CredentialType)> = connection
            .prepare(
                "SELECT signing_key, cred_type FROM as_signing_keys WHERE currently_active = true",
            )?
            .query_map([], |row| {
                let signing_key_bytes: Vec<u8> = row.get(0)?;
                let cred_type: CredentialType = row.get(1)?;
                Ok((signing_key_bytes, cred_type))
            })?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;

        let mut intermed_creds = vec![];
        let mut as_creds = vec![];
        for (signing_key_bytes, cred_type) in signing_key_bytes {
            match cred_type {
                CredentialType::As => {
                    let as_cred: AsSigningKey = phnxtypes::codec::from_slice(&signing_key_bytes)?;
                    as_creds.push(as_cred.credential().clone());
                }
                CredentialType::Intermediate => {
                    let intermed_cred: AsIntermediateSigningKey =
                        phnxtypes::codec::from_slice(&signing_key_bytes)?;
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
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        // There is currently only one OPAQUE setup.
        let opaque_setup =
            connection.query_row("SELECT opaque_setup FROM opaque_setup", [], |row| {
                let opaque_setup_bytes: Vec<u8> = row.get(0)?;
                let opaque_setup: ServerSetup<OpaqueCiphersuite> =
                    phnxtypes::codec::from_slice(&opaque_setup_bytes).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                    })?;
                Ok(opaque_setup)
            })?;

        Ok(opaque_setup)
    }

    // === Anonymous requests ===

    /// Return the client credentials of a user for a given username.
    async fn client_credentials(&self, user_name: &UserName) -> Vec<ClientCredential> {
        let Ok(user_name_bytes) = phnxtypes::codec::to_vec(user_name) else {
            return vec![];
        };
        let Ok(connection) = self.connection.lock() else {
            return vec![];
        };

        let Ok(mut statement) = connection
            .prepare("SELECT client_credential FROM as_client_records WHERE user_name = ?")
        else {
            return vec![];
        };

        let Ok(mapped_rows) = statement.query_map(params![user_name_bytes], |row| {
            let client_credential_bytes: Vec<u8> = row.get(0)?;
            let client_credential: ClientCredential =
                phnxtypes::codec::from_slice(&client_credential_bytes).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                })?;
            Ok(client_credential)
        }) else {
            return vec![];
        };

        let Ok(client_credentials) =
            mapped_rows.collect::<Result<Vec<ClientCredential>, rusqlite::Error>>()
        else {
            return vec![];
        };

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
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;

        let remaining_tokens: i16 = connection.query_row(
            "SELECT remaining_tokens FROM as_client_records WHERE client_id = ?",
            params![client_id.client_id()],
            |row| row.get(0),
        )?;

        // TODO: Unsafe conversion.
        Ok(remaining_tokens as usize)
    }

    async fn set_client_token_allowance(
        &self,
        client_id: &AsClientId,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;
        connection.execute(
            "UPDATE as_client_records SET remaining_tokens = ? WHERE client_id = ?",
            params![number_of_tokens as i16, client_id.client_id()],
        )?;

        Ok(())
    }

    /// Resets the token allowance of all clients. This should be called after a
    /// rotation of the privacy pass token issuance key material.
    async fn reset_token_allowances(
        &self,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| AsSqliteError::MutexError)?;
        connection.execute(
            "UPDATE as_client_records SET remaining_tokens = ?",
            params![number_of_tokens as i16],
        )?;
        Ok(())
    }
}
