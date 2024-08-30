// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use phnxbackend::qs::{
    client_record::QsClientRecord, storage_provider_trait::QsStorageProvider,
    user_record::QsUserRecord, QsConfig, QsSigningKey,
};
use phnxtypes::{
    crypto::{
        errors::RandomnessError, hpke::ClientIdDecryptionKey, signatures::keys::QsUserVerifyingKey,
    },
    identifiers::{Fqdn, QsClientId, QsUserId},
    keypackage_batch::QsEncryptedAddPackage,
    messages::{FriendshipToken, QueueMessage},
    time::TimeStamp,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use sqlx::types::Uuid;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SqliteQsStorage {
    connection: SqliteConnection,
    own_domain: Fqdn,
}

impl SqliteQsStorage {
    pub async fn new(path: &str, own_domain: Fqdn) -> Result<Self, CreateQsStorageError> {
        let connection = Connection::open(path)?;
        Self::initialize_db(connection, own_domain).await
    }

    pub async fn new_in_memory(own_domain: Fqdn) -> Result<Self, CreateQsStorageError> {
        let connection = Connection::open_in_memory()?;
        Self::initialize_db(connection, own_domain).await
    }

    async fn initialize_db(
        connection: Connection,
        own_domain: Fqdn,
    ) -> Result<Self, CreateQsStorageError> {
        let connection = Arc::new(Mutex::new(connection));

        let provider = Self {
            connection,
            own_domain,
        };

        provider.create_tables()?;

        // Check if the database has been initialized.

        // TODO: This should probably go into its own function and be made more
        // explicit and robust.
        if provider.load_decryption_key().await.is_err() {
            provider.generate_fresh_decryption_key().await?;
        }
        if provider.load_signing_key().await.is_err() {
            provider.generate_fresh_signing_key().await?;
        }
        if provider.load_config().await.is_err() {
            provider
                .store_config(QsConfig {
                    domain: provider.own_domain.clone(),
                })
                .await?;
        }

        Ok(provider)
    }

    fn create_tables(&self) -> Result<(), rusqlite::Error> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS qs_user_records (
                user_id BLOB PRIMARY KEY,
                friendship_token BLOB,
                verifying_key BLOB
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS qs_client_records (
                client_id BLOB PRIMARY KEY,
                user_id BLOB,
                encrypted_push_token BLOB,
                owner_public_key BLOB,
                owner_signature_key BLOB,
                ratchet BLOB,
                activity_time TEXT
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS key_packages (
                id BLOB PRIMARY KEY,
                client_id BLOB,
                encrypted_add_package BLOB,
                is_last_resort BOOLEAN
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS qs_signing_key (
                id BLOB PRIMARY KEY,
                signing_key BLOB
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS qs_decryption_key (
                id BLOB PRIMARY KEY,
                decryption_key BLOB
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS qs_config (
                id BLOB PRIMARY KEY,
                config BLOB
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS queues (
                message_id BLOB PRIMARY KEY,
                queue_id BLOB,
                sequence_number INTEGER,
                message_bytes BLOB
            )",
            [],
        )?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS queue_data (
                queue_id BLOB PRIMARY KEY,
                sequence_number INTEGER
            )",
            [],
        )?;
        Ok(())
    }

    // The following functions should probably be part of the QS storage provider trait.
    // TODO: All the functions below use two queries. This can probably be optimized.

    async fn generate_fresh_signing_key(&self) -> Result<(), GenerateKeyError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| GenerateKeyError::MutexError)?;
        // Delete the existing key.
        connection.execute("DELETE FROM qs_signing_key", [])?;

        // Generate a new one and add it to the table
        let signing_key = QsSigningKey::generate()?;
        connection.execute(
            "INSERT INTO qs_signing_key (id, signing_key) VALUES (?, ?)",
            params![Uuid::new_v4(), Cbor::to_vec(&signing_key)?],
        )?;
        Ok(())
    }

    async fn generate_fresh_decryption_key(&self) -> Result<(), GenerateKeyError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| GenerateKeyError::MutexError)?;

        // Delete the existing key.
        connection.execute("DELETE FROM qs_decryption_key", [])?;

        // Generate a new one and add it to the table
        let decryption_key = ClientIdDecryptionKey::generate()?;
        connection.execute(
            "INSERT INTO qs_decryption_key (id, decryption_key) VALUES (?, ?)",
            params![Uuid::new_v4(), Cbor::to_vec(&decryption_key)?],
        )?;

        Ok(())
    }

    async fn store_config(&self, config: QsConfig) -> Result<(), StoreConfigError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreConfigError::MutexError)?;
        // Delete the existing config.
        connection.execute("DELETE FROM qs_config", [])?;

        // Store the new config.
        connection.execute(
            "INSERT INTO qs_config (id, config) VALUES (?, ?)",
            params![Uuid::new_v4(), Cbor::to_vec(&config)?],
        )?;
        Ok(())
    }
}

#[async_trait]
impl QsStorageProvider for SqliteQsStorage {
    type EnqueueError = QueueError;
    type ReadAndDeleteError = ReadAndDeleteError;
    type CreateUserError = CreateUserError;
    type StoreUserError = StoreUserError;
    type DeleteUserError = DeleteUserError;
    type StoreClientError = StoreClientError;
    type CreateClientError = CreateClientError;
    type DeleteClientError = DeleteClientError;
    type StoreKeyPackagesError = StoreKeyPackagesError;

    type LoadSigningKeyError = LoadSigningKeyError;
    type LoadDecryptionKeyError = LoadDecryptionKeyError;

    type LoadConfigError = LoadConfigError;

    async fn own_domain(&self) -> Fqdn {
        self.own_domain.clone()
    }

    async fn create_user(
        &self,
        user_record: QsUserRecord,
    ) -> Result<QsUserId, Self::CreateUserError> {
        let user_id = QsUserId::random();
        let connection = self
            .connection
            .lock()
            .map_err(|_| CreateUserError::MutexError)?;
        connection.execute(
            "INSERT INTO qs_user_records (user_id, friendship_token, verifying_key) VALUES (?, ?, ?)",
            params![
                user_id.as_uuid(),
                user_record.friendship_token().token(),
                user_record.verifying_key().as_ref(),
            ],
        )?;
        Ok(user_id)
    }

    async fn load_user(&self, user_id: &QsUserId) -> Option<QsUserRecord> {
        let connection = self.connection.lock().ok()?;
        connection
            .query_row(
                "SELECT * FROM qs_user_records WHERE user_id = ?",
                params![user_id.as_uuid()],
                |row| {
                    let verifying_key = row.get::<_, Vec<u8>>(2)?;
                    let friendship_token = row.get::<_, Vec<u8>>(1)?;
                    Ok(QsUserRecord::new(
                        QsUserVerifyingKey::from_bytes(verifying_key),
                        FriendshipToken::from_bytes(friendship_token),
                    ))
                },
            )
            .optional()
            .ok()
            .flatten()
    }

    async fn store_user(
        &self,
        user_id: &QsUserId,
        user_record: QsUserRecord,
    ) -> Result<(), Self::StoreUserError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreUserError::MutexError)?;
        connection.execute(
            "UPDATE qs_user_records SET friendship_token = ?, verifying_key = ? WHERE user_id = ?",
            params![
                user_record.friendship_token().token(),
                user_record.verifying_key().as_ref(),
                user_id.as_uuid(),
            ],
        )?;
        Ok(())
    }

    async fn delete_user(&self, user_id: &QsUserId) -> Result<(), Self::DeleteUserError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| DeleteUserError::MutexError)?;
        connection.execute(
            "DELETE FROM qs_user_records WHERE user_id = ?",
            params![user_id.as_uuid()],
        )?;

        // Get all client ids of the user s.t. we can delete the queues as well.
        let mut statement =
            connection.prepare("SELECT client_id FROM qs_client_records WHERE user_id = ?")?;
        let client_records = statement
            .query_map(params![user_id.as_uuid()], |row| row.get::<_, Uuid>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for client_record in client_records {
            connection.execute(
                "DELETE FROM queue_data WHERE queue_id = ?",
                params![client_record],
            )?;
            connection.execute(
                "DELETE FROM key_packages WHERE client_id = ?",
                params![client_record],
            )?;
        }
        // Delete all the client data.
        connection.execute(
            "DELETE FROM qs_client_records WHERE user_id = ?",
            params![user_id.as_uuid()],
        )?;

        Ok(())
    }

    async fn create_client(
        &self,
        client_record: QsClientRecord,
    ) -> Result<QsClientId, Self::CreateClientError> {
        let client_id = QsClientId::random();

        // Create and store the client record.
        let encrypted_push_token = if let Some(ept) = client_record.encrypted_push_token() {
            Some(Cbor::to_vec(ept)?)
        } else {
            None
        };
        let owner_public_key = Cbor::to_vec(client_record.owner_public_key())?;
        let owner_signature_key = Cbor::to_vec(client_record.owner_signature_key())?;
        let ratchet = Cbor::to_vec(client_record.current_ratchet_key())?;
        let activity_time = client_record.activity_time().time();
        let connection = self
            .connection
            .lock()
            .map_err(|_| CreateClientError::MutexError)?;
        connection.execute(
            "INSERT INTO qs_client_records (client_id, user_id, encrypted_push_token, owner_public_key, owner_signature_key, ratchet, activity_time) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                client_id.as_uuid(),
                client_record.user_id().as_uuid(),
                encrypted_push_token,
                owner_public_key,
                owner_signature_key,
                ratchet,
                activity_time,
            ],
        )?;

        // Initialize the client's queue
        connection.execute(
            "INSERT INTO queue_data (queue_id, sequence_number) VALUES (?, ?)",
            params![client_id.as_uuid(), 0u64],
        )?;

        Ok(client_id)
    }

    async fn load_client(&self, client_id: &QsClientId) -> Option<QsClientRecord> {
        let connection = self.connection.lock().ok()?;
        let res = connection
            .query_row(
                "SELECT * FROM qs_client_records WHERE client_id = ?",
                params![client_id.as_uuid()],
                |row| {
                    let user_id = QsUserId::from(row.get::<_, Uuid>(1)?);
                    let encrypted_push_token = if let Some(ept) =
                        row.get::<_, Option<Vec<u8>>>(2)?
                    {
                        Some(Cbor::from_slice(&ept).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(2, Type::Blob, Box::new(e))
                        })?)
                    } else {
                        None
                    };
                    let owner_public_key = Cbor::from_slice(&row.get::<_, Vec<u8>>(3)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(3, Type::Blob, Box::new(e))
                        })?;
                    let owner_signature_key = Cbor::from_slice(&row.get::<_, Vec<u8>>(4)?)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(4, Type::Blob, Box::new(e))
                        })?;
                    let ratchet =
                        Cbor::from_slice(&row.get::<_, Vec<u8>>(5)?).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(5, Type::Blob, Box::new(e))
                        })?;
                    let activity_time_raw = row.get::<_, DateTime<Utc>>(6)?;
                    let activity_time = TimeStamp::from(activity_time_raw);
                    Ok(QsClientRecord::from_db_values(
                        user_id,
                        encrypted_push_token,
                        owner_public_key,
                        owner_signature_key,
                        ratchet,
                        activity_time,
                    ))
                },
            )
            .optional();
        match res {
            Err(e) => {
                tracing::warn!("Error loading client: {:?}", e);
                None
            }
            Ok(client_record) => client_record,
        }
    }

    async fn store_client(
        &self,
        client_id: &QsClientId,
        client_record: QsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        let encrypted_push_token = if let Some(ept) = client_record.encrypted_push_token() {
            Some(Cbor::to_vec(ept)?)
        } else {
            None
        };
        let owner_public_key = Cbor::to_vec(client_record.owner_public_key())?;
        let owner_signature_key = Cbor::to_vec(client_record.owner_signature_key())?;
        let ratchet = Cbor::to_vec(client_record.current_ratchet_key())?;
        let activity_time = client_record.activity_time().time();
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreClientError::MutexError)?;
        connection.execute(
            "UPDATE qs_client_records SET user_id = ?, encrypted_push_token = ?, owner_public_key = ?, owner_signature_key = ?, ratchet = ?, activity_time = ? WHERE client_id = ?",
            params![
                client_record.user_id().as_uuid(),
                encrypted_push_token,
                owner_public_key,
                owner_signature_key,
                ratchet,
                activity_time,
                client_id.as_uuid(),
            ],
        )?;

        Ok(())
    }

    async fn delete_client(&self, client_id: &QsClientId) -> Result<(), Self::DeleteClientError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| DeleteClientError::MutexError)?;
        connection.execute(
            "DELETE FROM qs_client_records WHERE client_id = ?",
            params![client_id.as_uuid()],
        )?;
        connection.execute(
            "DELETE FROM queue_data WHERE queue_id = ?",
            params![client_id.as_uuid()],
        )?;
        connection.execute(
            "DELETE FROM key_packages WHERE client_id = ?",
            params![client_id.as_uuid()],
        )?;
        Ok(())
    }

    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        // TODO: This can probably be improved. For now, we insert each key
        // package individually.
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreKeyPackagesError::MutexError)?;
        for kp in encrypted_key_packages {
            store_key_package(&connection, client_id, kp, false)?;
        }
        Ok(())
    }

    async fn store_last_resort_key_package(
        &self,
        client_id: &QsClientId,
        encrypted_key_package: QsEncryptedAddPackage,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreKeyPackagesError::MutexError)?;
        store_key_package(&connection, client_id, encrypted_key_package, true)?;
        Ok(())
    }

    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage> {
        let connection = self.connection.lock().ok()?;
        // Check if the given client belongs to the given user.
        let _client_record = connection
            .query_row(
                "SELECT * FROM qs_client_records WHERE client_id = ? AND user_id = ?",
                params![client_id.as_uuid(), user_id.as_uuid()],
                |_| Ok(()),
            )
            .optional()
            .ok()?;
        let (id, add_package) = connection
            .query_row(
                "SELECT id, encrypted_add_package FROM key_packages WHERE client_id = ?",
                params![client_id.as_uuid()],
                |row| {
                    let id = row.get::<_, Uuid>(0)?;
                    let ciphertext_bytes = row.get::<_, Vec<u8>>(1)?;
                    let ciphertext = Cbor::from_slice(&ciphertext_bytes).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, Box::new(e))
                    })?;
                    Ok((id, ciphertext))
                },
            )
            .optional()
            .ok()??;
        connection
            .execute("DELETE FROM key_packages WHERE id = ?", params![id])
            .ok()?;

        Some(add_package)
    }

    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Vec<QsEncryptedAddPackage> {
        let Ok(connection) = self.connection.lock() else {
            return vec![];
        };
        // Figure out which user corresponds to the friendship token
        let Ok(query_result) = connection.query_row(
            "SELECT user_id FROM qs_user_records WHERE friendship_token = ?",
            params![friendship_token.token()],
            |row| Ok(row.get::<_, Uuid>(0)),
        ) else {
            return vec![];
        };
        let Ok(user_records) = query_result else {
            return vec![];
        };

        // Figure out which clients the user has
        let Ok(mut statement) =
            connection.prepare("SELECT client_id FROM qs_client_records WHERE user_id = ?")
        else {
            return vec![];
        };
        let Ok(client_records_result) =
            statement.query_map(params![user_records], |row| Ok(row.get::<_, Uuid>(0)))
        else {
            return vec![];
        };
        let Ok(client_records) = client_records_result.collect::<Result<Vec<_>, _>>() else {
            return vec![];
        };

        // Get a key package for each client.
        // TODO: Again, this can probably be optimized
        let mut add_packages: Vec<QsEncryptedAddPackage> = vec![];
        for client_id in client_records {
            let Ok(client_id) = client_id else { continue };
            let Ok((id, add_package)) = connection.query_row(
                "SELECT id, encrypted_add_package FROM key_packages WHERE client_id = ?",
                params![client_id],
                |row| {
                    let id = row.get::<_, Uuid>(0)?;
                    let ciphertext_bytes = row.get::<_, Vec<u8>>(1)?;
                    let ciphertext = Cbor::from_slice(&ciphertext_bytes).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(1, Type::Blob, Box::new(e))
                    })?;
                    Ok((id, ciphertext))
                },
            ) else {
                continue;
            };
            let _ = connection.execute("DELETE FROM key_packages WHERE id = ?", params![id]);

            add_packages.push(add_package);
        }

        add_packages
    }

    // TODO: The whole queueing scheme can probably be optimized quite a bit.
    async fn enqueue(
        &self,
        client_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError> {
        let connection = self.connection.lock().map_err(|_| QueueError::MutexError)?;
        // Check if sequence numbers are consistent.
        let sequence_number = connection.query_row(
            "SELECT sequence_number FROM queue_data WHERE queue_id = ?",
            params![client_id.as_uuid()],
            |row| row.get::<_, u64>(0),
        )?;

        if sequence_number != message.sequence_number {
            tracing::warn!(
                "Sequence number mismatch. Message sequence number {}, queue sequence number {}",
                message.sequence_number,
                sequence_number
            );
            return Err(QueueError::SequenceNumberMismatch);
        }

        // Get a fresh message ID (only used as a unique key for Sqlite)
        let message_id = Uuid::new_v4();
        let message_bytes = Cbor::to_vec(&message)?;
        // Store the message in the DB
        connection.execute(
            "INSERT INTO queues (message_id, queue_id, sequence_number, message_bytes) VALUES (?, ?, ?, ?)",
            params![message_id, client_id.as_uuid(), message.sequence_number, message_bytes],
        )?;

        let new_sequence_number = sequence_number + 1;
        // Increase the sequence number and store it.
        connection.execute(
            "UPDATE queue_data SET sequence_number = ? WHERE queue_id = ?",
            params![new_sequence_number, client_id.as_uuid()],
        )?;

        Ok(())
    }

    async fn read_and_delete(
        &self,
        client_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError> {
        // TODO: We can probably combine these three queries into one.
        let connection = self
            .connection
            .lock()
            .map_err(|_| ReadAndDeleteError::MutexError)?;

        // Delete all messages until the given "last seen" one.
        connection.execute(
            "DELETE FROM queues WHERE queue_id = ? AND sequence_number < ?",
            params![client_id.as_uuid(), sequence_number],
        )?;

        // Now fetch at most `number_of_messages` messages from the queue.

        // TODO: sqlx wants an i64 here and in a few other places below, but
        // we're using u64s. This is probably a limitation of Sqlite and we
        // might want to change some of the input/output types accordingly.
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| ReadAndDeleteError::LibraryError)?;
        let mut statement = connection.prepare(
            "SELECT message_bytes FROM queues WHERE queue_id = ? ORDER BY sequence_number ASC LIMIT ?",
        )?;
        let messages: Vec<QueueMessage> = statement
            .query_map(params![client_id.as_uuid(), number_of_messages], |row| {
                let message_bytes = row.get::<_, Vec<u8>>(0)?;
                let message = Cbor::from_slice(&message_bytes).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                })?;
                Ok(message)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let lower_limit = sequence_number + messages.len() as u64;
        let remaining_messages = connection.query_row(
            "SELECT COUNT(*) as count FROM queues WHERE queue_id = ? AND sequence_number >= ?",
            params![client_id.as_uuid(), lower_limit],
            |row| row.get::<_, i64>(0),
        )?;

        return Ok((messages, remaining_messages as u64));
    }

    async fn load_signing_key(&self) -> Result<QsSigningKey, Self::LoadSigningKeyError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LoadSigningKeyError::MutexError)?;
        let signing_key =
            connection.query_row("SELECT signing_key FROM qs_signing_key", [], |row| {
                let signing_key_bytes = row.get::<_, Vec<u8>>(0)?;
                let signing_key = Cbor::from_slice(&signing_key_bytes).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                })?;
                Ok(signing_key)
            })?;
        Ok(signing_key)
    }

    async fn load_decryption_key(
        &self,
    ) -> Result<ClientIdDecryptionKey, Self::LoadDecryptionKeyError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LoadDecryptionKeyError::MutexError)?;
        let decryption_key =
            connection.query_row("SELECT decryption_key FROM qs_decryption_key", [], |row| {
                let decryption_key_bytes = row.get::<_, Vec<u8>>(0)?;
                let decryption_key =
                    Cbor::from_slice(&decryption_key_bytes).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
                    })?;
                Ok(decryption_key)
            })?;
        Ok(decryption_key)
    }

    async fn load_config(&self) -> Result<QsConfig, Self::LoadConfigError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| LoadConfigError::MutexError)?;
        let config = connection.query_row("SELECT config FROM qs_config", [], |row| {
            let config_bytes = row.get::<_, Vec<u8>>(0)?;
            let config = Cbor::from_slice(&config_bytes).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Blob, Box::new(e))
            })?;
            Ok(config)
        })?;
        Ok(config)
    }
}

fn store_key_package(
    connection: &Connection,
    client_id: &QsClientId,
    encrypted_key_package: QsEncryptedAddPackage,
    is_last_resort: bool,
) -> Result<(), StoreKeyPackagesError> {
    // TODO: This can probably be improved. For now, we insert each key
    // package individually.
    let id = Uuid::new_v4();
    let client_uuid = client_id.as_uuid();
    let ciphertext_bytes = Cbor::to_vec(&encrypted_key_package)?;
    connection.execute(
        "INSERT INTO key_packages (id, client_id, encrypted_add_package, is_last_resort) VALUES (?, ?, ?, ?)",
        params![id, client_uuid, ciphertext_bytes, is_last_resort],
    )?;
    Ok(())
}

#[derive(Error, Debug)]
#[repr(u8)]
pub enum StoreUserError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
}
#[derive(Error, Debug)]
pub enum DeleteUserError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
}
#[derive(Error, Debug)]
pub enum StoreClientError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Error serializing client record
    #[error(transparent)]
    SerializationError(#[from] Cbor::Error),
}

#[derive(Error, Debug)]
#[repr(u8)]
pub enum CreateClientError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
    /// Error serializing client record
    #[error(transparent)]
    SerializationError(#[from] Cbor::Error),
}

#[derive(Error, Debug)]
pub enum DeleteClientError {
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
    /// Unknown client.
    #[error("Unknown client.")]
    UnknownClient,
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
}

#[derive(Error, Debug)]
pub enum StoreKeyPackagesError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Unknown client.
    #[error("Unknown client.")]
    UnknownClient,
    /// Error serializing KeyPackage
    #[error(transparent)]
    SerializationError(#[from] Cbor::Error),
}

/// Error creating user
#[derive(Error, Debug)]
pub enum CreateUserError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
}

/// Error creating queue
#[derive(Error, Debug)]
pub enum CreateQueueError {
    /// The given queue id collides with an existing one.
    #[error("The given queue id collides with an existing one.")]
    QueueIdCollision,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
}

/// General error while accessing the requested queue.
#[derive(Error, Debug)]
pub enum QueueError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Error serializing message
    #[error(transparent)]
    SerializationError(#[from] Cbor::Error),
}

/// Error while trying to read and delete messages from queue.
#[derive(Error, Debug)]
pub enum ReadAndDeleteError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Error deserializing message
    #[error(transparent)]
    DeserializationError(#[from] Cbor::Error),
    /// A queue with the given id could not be found.
    #[error("The given queue id collides with an existing one.")]
    QueueNotFound,
    /// The given sequence number could not be found in the queue.
    #[error("The given sequence number could not be found in the queue.")]
    SequenceNumberNotFound,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
}

#[derive(Error, Debug)]
pub enum GenerateKeyError {
    #[error("Mutex poisoned")]
    MutexError,
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] Cbor::Error),
    #[error(transparent)]
    RandomnessError(#[from] RandomnessError),
}

#[derive(Error, Debug)]
pub enum LoadSigningKeyError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] Cbor::Error),
}

#[derive(Error, Debug)]
pub enum LoadDecryptionKeyError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] Cbor::Error),
}

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Error deserializing key
    #[error(transparent)]
    DeserializationError(#[from] Cbor::Error),
}

#[derive(Error, Debug)]
pub enum StoreConfigError {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
    /// Error deserializing key
    #[error(transparent)]
    SerializationError(#[from] Cbor::Error),
}

#[derive(Error, Debug)]
pub enum CreateQsStorageError {
    #[error(transparent)]
    StoreConfigError(#[from] StoreConfigError),
    #[error(transparent)]
    GenerateKeyError(#[from] GenerateKeyError),
    #[error(transparent)]
    StorageError(#[from] rusqlite::Error),
    #[error("Mutex poisoned")]
    MutexError,
}
