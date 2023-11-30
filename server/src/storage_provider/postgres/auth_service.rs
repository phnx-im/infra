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
    batched_tokens::server::BatchedKeyStore,
    private_tokens::{Ristretto255, VoprfServer},
    TokenKeyId,
};
use sqlx::{
    types::{BigDecimal, Uuid},
    Connection, Executor, PgConnection, PgPool,
};
use thiserror::Error;

pub struct PostgresAsStorage {
    pool: PgPool,
}

impl PostgresAsStorage {
    pub async fn new(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
        settings: &DatabaseSettings,
    ) -> Result<Self, CreateAsStorageError> {
        // Create database
        let mut connection =
            PgConnection::connect(&settings.connection_string_without_database()).await?;
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, settings.database_name).as_str())
            .await?;
        // Migrate database
        let connection_pool = PgPool::connect(&settings.connection_string()).await?;
        sqlx::migrate!("./migrations").run(&connection_pool).await?;

        let provider = Self {
            pool: connection_pool,
        };

        // Check if the database has been initialized.
        let (as_creds, _as_inter_creds, _) = provider.load_as_credentials().await?;
        if as_creds.is_empty() {
            let (as_signing_key, as_inter_signing_key) =
                Self::generate_fresh_credentials(as_domain, signature_scheme)?;
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

    fn generate_fresh_credentials(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
    ) -> Result<(AsSigningKey, AsIntermediateSigningKey), CreateAsStorageError> {
        let (_credential, as_signing_key) =
            AsCredential::new(signature_scheme, as_domain.clone(), None)
                .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
        let (csr, prelim_signing_key) =
            AsIntermediateCredentialCsr::new(signature_scheme, as_domain)
                .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
        let as_intermediate_credential = csr
            .sign(&as_signing_key, None)
            .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
        let as_intermediate_signing_key = AsIntermediateSigningKey::from_prelim_key(
            prelim_signing_key,
            as_intermediate_credential,
        )
        .map_err(|_| CreateAsStorageError::CredentialGenerationError)?;
        Ok((as_signing_key, as_intermediate_signing_key))
    }
}

#[async_trait]
impl BatchedKeyStore for PostgresAsStorage {
    /// Inserts a keypair with a given `token_key_id` into the key store.
    async fn insert(&self, token_key_id: TokenKeyId, server: VoprfServer<Ristretto255>) {
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
    async fn get(&self, token_key_id: &TokenKeyId) -> Option<VoprfServer<Ristretto255>> {
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
enum CredentialType {
    As,
    Intermediate,
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
            "INSERT INTO queue_data (queue_id, sequence_number) VALUES ($1, $2)",
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
        // TODO: This can probably be improved. For now, we insert each connection
        // package individually.
        for connection_package in connection_packages {
            let id = Uuid::new_v4();
            let connection_package_bytes = serde_json::to_vec(&connection_package)?;
            sqlx::query!(
                "INSERT INTO connection_packages (id, client_id, connection_package) VALUES ($1, $2, $3)",
                id,
                client_id.client_id(),
                connection_package_bytes,
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Return a key package for a specific client. The client_id must belong to
    /// the same user as the requested key packages.
    /// TODO: Last resort key package
    async fn client_connection_package(&self, client_id: &AsClientId) -> Option<ConnectionPackage> {
        let connection_package_bytes_record = sqlx::query!(
            "SELECT id, connection_package FROM connection_packages WHERE client_id = $1",
            client_id.client_id(),
        )
        .fetch_one(&self.pool)
        .await
        .ok()?;
        let connection_package =
            serde_json::from_slice(&connection_package_bytes_record.connection_package).ok()?;

        // If there is only one left, leave it. Otherwise, delete it.
        let remaining_add_packages = sqlx::query!(
            "SELECT COUNT(*) as count FROM connection_packages WHERE client_id = $1",
            client_id.client_id(),
        )
        .fetch_one(&self.pool)
        .await
        .ok()?
        .count?;

        if remaining_add_packages > 1 {
            sqlx::query!(
                "DELETE FROM connection_packages WHERE id = $1",
                connection_package_bytes_record.id,
            )
            .execute(&self.pool)
            .await
            .ok()?;
        };

        Some(connection_package)
    }

    /// Return a key package for each client of a user referenced by a
    /// user name.
    async fn load_user_connection_packages(
        &self,
        user_name: &UserName,
    ) -> Result<Vec<ConnectionPackage>, Self::StorageError> {
        let user_name_bytes = serde_json::to_vec(user_name)?;
        // Collect all client ids associated with that user.
        let client_ids_record = sqlx::query!(
            "SELECT client_id FROM as_client_records WHERE user_name = $1",
            user_name_bytes
        )
        .fetch_all(&self.pool)
        .await?;
        let mut connection_packages = Vec::new();
        for client_id in client_ids_record {
            let connection_package_record = sqlx::query!(
                "SELECT id, connection_package FROM connection_packages WHERE client_id = $1",
                client_id.client_id,
            )
            .fetch_one(&self.pool)
            .await?;
            sqlx::query!(
                "DELETE FROM connection_packages WHERE id = $1",
                connection_package_record.id,
            )
            .execute(&self.pool)
            .await?;

            let connection_package =
                serde_json::from_slice(&connection_package_record.connection_package)?;
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
        // Check if sequence numbers are consistent.
        let sequence_number_record = sqlx::query!(
            "SELECT sequence_number FROM queue_data WHERE queue_id = $1",
            client_id.client_id(),
        )
        .fetch_one(&self.pool)
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
        let message_bytes = serde_json::to_vec(&message)?;
        // Store the message in the DB
        sqlx::query!(
            "INSERT INTO queues (message_id, queue_id, sequence_number, message_bytes) VALUES ($1, $2, $3, $4)",
            message_id,
            client_id.client_id(),
            sequence_number_decimal,
            message_bytes,
        )
        .execute(&self.pool)
        .await?;

        let new_sequence_number = sequence_number_decimal + BigDecimal::from(1u8);
        // Increase the sequence number and store it.
        sqlx::query!(
            "UPDATE queue_data SET sequence_number = $2 WHERE queue_id = $1",
            client_id.client_id(),
            new_sequence_number
        )
        .execute(&self.pool)
        .await?;
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
        // TODO: We can probably combine these three queries into one.

        // Delete all messages until the given "last seen" one.
        sqlx::query!(
            "DELETE FROM queues WHERE queue_id = $1 AND sequence_number < $2",
            client_id.client_id(),
            sequence_number_decimal,
        )
        .execute(&self.pool)
        .await?;

        // Now fetch at most `number_of_messages` messages from the queue.

        // TODO: sqlx wants an i64 here and in a few other places below, but
        // we're using u64s. This is probably a limitation of postgres and we
        // might want to change some of the input/output types accordingly.
        let number_of_messages =
            i64::try_from(number_of_messages).map_err(|_| QueueError::LibraryError)?;
        let records = sqlx::query!(
            "SELECT message_bytes FROM queues WHERE queue_id = $1 ORDER BY sequence_number ASC LIMIT $2",
            client_id.client_id(),
            number_of_messages,
        )
        .fetch_all(&self.pool)
        .await?;

        let lower_limit = BigDecimal::from(sequence_number + records.len() as u64);
        let remaining_messages = sqlx::query!(
            "SELECT COUNT(*) as count FROM queues WHERE queue_id = $1 AND sequence_number >= $2 ",
            client_id.client_id(),
            lower_limit,
        )
        .fetch_one(&self.pool)
        .await?
        .count
        // Count should return something.
        .ok_or(QueueError::LibraryError)?;

        // Convert the records to messages.
        let messages = records
            .into_iter()
            .map(|record| {
                let message = serde_json::from_slice(&record.message_bytes)?;
                Ok(message)
            })
            .collect::<Result<Vec<_>, QueueError>>()?;

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
