// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, VecDeque},
    sync::RwLock,
};

use async_trait::async_trait;
use mls_assist::openmls_traits::types::SignatureScheme;
use opaque_ke::{rand::rngs::OsRng, ServerLogin, ServerRegistration, ServerSetup};
use phnxbackend::auth_service::{
    storage_provider_trait::{AsEphemeralStorageProvider, AsStorageProvider},
    AsClientRecord, AsUserRecord,
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
};
use privacypass_middleware::memory_stores::MemoryKeyStore;
use thiserror::Error;

use super::qs::QueueData;

pub struct MemoryAsStorage {
    user_records: RwLock<HashMap<UserName, AsUserRecord>>,
    client_records: RwLock<HashMap<AsClientId, AsClientRecord>>,
    connection_packages: RwLock<HashMap<AsClientId, VecDeque<ConnectionPackage>>>,
    queues: RwLock<HashMap<AsClientId, QueueData>>,
    as_intermediate_signing_key: RwLock<AsIntermediateSigningKey>,
    as_signing_key: RwLock<AsSigningKey>,
    opaque_server_setup: RwLock<ServerSetup<OpaqueCiphersuite>>,
    // No RwLock needed, as MemoryKeyStore is already thread-safe.
    privacy_pass_key_store: MemoryKeyStore,
    remaining_tokens: RwLock<HashMap<AsClientId, usize>>,
}

#[derive(Debug, Error, Clone)]
pub enum StorageInitError {
    /// Credential generation error.
    #[error("Credential generation error.")]
    CredentialGenerationError,
}

impl MemoryAsStorage {
    pub fn new(
        as_domain: Fqdn,
        signature_scheme: SignatureScheme,
    ) -> Result<Self, StorageInitError> {
        let mut rng = OsRng;
        let opaque_server_setup = RwLock::new(ServerSetup::<OpaqueCiphersuite>::new(&mut rng));
        let (_credential, as_signing_key) =
            AsCredential::new(signature_scheme, as_domain.clone(), None)
                .map_err(|_| StorageInitError::CredentialGenerationError)?;
        let (csr, prelim_signing_key) =
            AsIntermediateCredentialCsr::new(signature_scheme, as_domain)
                .map_err(|_| StorageInitError::CredentialGenerationError)?;
        let as_intermediate_credential = csr
            .sign(&as_signing_key, None)
            .map_err(|_| StorageInitError::CredentialGenerationError)?;
        let as_intermediate_signing_key = RwLock::new(
            AsIntermediateSigningKey::from_prelim_key(
                prelim_signing_key,
                as_intermediate_credential,
            )
            .map_err(|_| StorageInitError::CredentialGenerationError)?,
        );
        let privacy_pass_key_store = MemoryKeyStore::default();

        let as_signing_key = RwLock::new(as_signing_key);
        let storage_provider = Self {
            user_records: RwLock::new(HashMap::new()),
            client_records: RwLock::new(HashMap::new()),
            connection_packages: RwLock::new(HashMap::new()),
            queues: RwLock::new(HashMap::new()),
            as_intermediate_signing_key,
            as_signing_key,
            opaque_server_setup,
            privacy_pass_key_store,
            remaining_tokens: RwLock::new(HashMap::new()),
        };
        Ok(storage_provider)
    }
}

#[derive(Debug, Error, Clone)]
pub enum AsQueueError {
    /// Lock poisoned
    #[error("Lock poisoned")]
    PoisonedLock,
    /// Queue not found.
    #[error("Queue not found.")]
    QueueNotFound,
    /// Mismatching sequence number.
    #[error("Mismatching sequence number.")]
    SequenceNumberMismatch,
}

#[derive(Debug, Error, Clone)]
pub enum ReadAndDeleteError {
    /// Lock poisoned
    #[error("Lock poisoned")]
    PoisonedLock,
    /// Queue not found.
    #[error("Queue not found.")]
    QueueNotFound,
    /// Sequence number not found.
    #[error("Sequence number not found.")]
    SequenceNumberNotFound,
}

#[derive(Debug, Error, Clone)]
pub enum AsStorageError {
    /// Lock poisoned
    #[error("Lock poisoned")]
    PoisonedLock,
}

#[derive(Debug, Error, Clone)]
pub enum AsCreateClientError {
    #[error(transparent)]
    AsStorageError(#[from] AsStorageError),
    #[error("Client already exists.")]
    DuplicateClientId,
}

const DEFAULT_NUMBER_OF_TOKENS: usize = 100;

#[async_trait]
impl AsStorageProvider for MemoryAsStorage {
    type PrivacyPassKeyStore = MemoryKeyStore;
    type StorageError = AsStorageError;

    type CreateUserError = AsStorageError;
    type StoreUserError = AsStorageError;
    type DeleteUserError = AsStorageError;

    type StoreClientError = AsStorageError;
    type CreateClientError = AsCreateClientError;
    type DeleteClientError = AsStorageError;

    type EnqueueError = AsQueueError;
    type ReadAndDeleteError = ReadAndDeleteError;

    type StoreKeyPackagesError = AsStorageError;

    type LoadSigningKeyError = AsStorageError;
    type LoadAsCredentialsError = AsStorageError;

    type LoadOpaqueKeyError = AsStorageError;

    // === Users ===

    /// Loads the AsUserRecord for a given UserName. Returns None if no AsUserRecord
    /// exists for the given UserId.
    async fn load_user(&self, user_name: &UserName) -> Option<AsUserRecord> {
        self.user_records.read().ok()?.get(user_name).cloned()
    }

    /// Create a new user with the given user name. If a user with the given user
    /// name already exists, an error is returned.
    async fn create_user(
        &self,
        user_name: &UserName,
        opaque_record: &ServerRegistration<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError> {
        let user_record = AsUserRecord::new(user_name.clone(), opaque_record.clone());
        self.user_records
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .insert(user_name.clone(), user_record);
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
        let client_ids: Vec<AsClientId> = self
            .client_records
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .keys()
            .filter(|client_id| &client_id.user_name() == user_id)
            .cloned()
            .collect();
        // Delete all the user's clients. The user will be deleted with its last
        // client.
        for client_id in client_ids {
            self.delete_client(&client_id).await?;
        }
        Ok(())
    }

    // === Clients ===

    async fn create_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::CreateClientError> {
        let mut clients = self
            .client_records
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?;
        if clients.contains_key(client_id) {
            return Err(AsCreateClientError::DuplicateClientId);
        }
        clients.insert(client_id.clone(), client_record.clone());
        // If the client is first created, also create a queue and a token
        // allowance.
        self.queues
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .insert(client_id.clone(), QueueData::new());
        self.remaining_tokens
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .insert(client_id.clone(), DEFAULT_NUMBER_OF_TOKENS);
        Ok(())
    }

    /// Load the info for the client with the given client ID.
    async fn load_client(&self, client_id: &AsClientId) -> Option<AsClientRecord> {
        self.client_records.read().ok()?.get(client_id).cloned()
    }

    /// Saves a client in the storage provider with the given client ID. The
    /// storage provider must associate this client with the user of the client.
    async fn store_client(
        &self,
        client_id: &AsClientId,
        client_record: &AsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        let new_client = self
            .client_records
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .insert(client_id.clone(), client_record.clone())
            .is_none();
        // If the client is first created, also create a queue and a token
        // allowance.
        if new_client {
            self.queues
                .write()
                .map_err(|_| AsStorageError::PoisonedLock)?
                .insert(client_id.clone(), QueueData::new());
            self.remaining_tokens
                .write()
                .map_err(|_| AsStorageError::PoisonedLock)?
                .insert(client_id.clone(), DEFAULT_NUMBER_OF_TOKENS);
        }
        Ok(())
    }

    /// Deletes the client with the given client ID.
    ///
    /// The storage provider must also delete the following:
    ///  - The associated user, if the user has no other clients
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_client(&self, client_id: &AsClientId) -> Result<(), Self::StorageError> {
        self.client_records
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .remove(client_id);
        self.connection_packages
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .remove(client_id);
        self.queues
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .remove(client_id);
        self.remaining_tokens
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .remove(client_id);
        // If there are now more clients for the user, delete the user.
        let no_more_clients = self
            .client_records
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .keys()
            .all(|id| id.user_name() != client_id.user_name());
        if no_more_clients {
            let user_id = client_id.user_name();
            self.delete_user(&user_id).await?;
        }
        Ok(())
    }

    // === Key packages ===

    /// Store key packages for a specific client.
    async fn store_connection_packages(
        &self,
        client_id: &AsClientId,
        connection_packages: Vec<ConnectionPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        self.connection_packages
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .insert(client_id.clone(), connection_packages.into());
        Ok(())
    }

    /// Return a key package for a specific client. The client_id must belong to
    /// the same user as the requested key packages.
    /// TODO: Last resort key package
    async fn client_connection_package(&self, client_id: &AsClientId) -> Option<ConnectionPackage> {
        let mut connection_package_store = self.connection_packages.write().ok()?;

        let connection_packages = connection_package_store.get_mut(client_id)?;
        if connection_packages.len() == 1 {
            connection_packages.front().cloned()
        } else {
            connection_packages.pop_front()
        }
    }

    /// Return a key package for each client of a user referenced by a
    /// user name.
    async fn load_user_connection_packages(
        &self,
        user_name: &UserName,
    ) -> Result<Vec<ConnectionPackage>, Self::StorageError> {
        let client_records: Vec<_> = self
            .client_records
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)?
            .keys()
            .cloned()
            .collect();
        let mut connection_packages = Vec::new();
        for client_id in &client_records {
            if &client_id.user_name() == user_name {
                if let Some(connection_package) = self.client_connection_package(client_id).await {
                    connection_packages.push(connection_package);
                } else {
                    tracing::warn!(
                        "Did not find connection package for client with id {:?}.",
                        client_id
                    );
                }
            }
        }
        Ok(connection_packages)
    }

    // === Messages ===

    // --- Legacy ---

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    async fn enqueue(
        &self,
        client_id: &AsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError> {
        let mut queues = self
            .queues
            .write()
            .map_err(|_| AsQueueError::PoisonedLock)?;
        let queue = queues
            .get_mut(client_id)
            .ok_or(AsQueueError::QueueNotFound)?;

        // Check if sequence numbers are consistent.
        if queue.sequence_number != message.sequence_number {
            tracing::warn!("Inconsistent sequence numbers: Queue sequence number: {}, message sequence number: {}", queue.sequence_number, message.sequence_number);
            return Err(AsQueueError::SequenceNumberMismatch);
        }
        queue.sequence_number += 1;
        queue.queue.push_back(message);
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
        let mut queues = self
            .queues
            .write()
            .map_err(|_| ReadAndDeleteError::PoisonedLock)?;
        let queue = queues
            .get_mut(client_id)
            .ok_or(ReadAndDeleteError::QueueNotFound)?;

        if number_of_messages == 0 {
            // Converting usize to u64 should be safe since we don't consider
            // architectures above 64.
            return Ok((vec![], queue.queue.len() as u64));
        }

        let mut return_messages = vec![];
        while let Some(first_message) = queue.queue.pop_front() {
            if first_message.sequence_number >= sequence_number {
                // If we're above the "last seen" sequence number given by the
                // client, add the popped message to the messages to be
                // returned.
                // Messages with a lower sequence number are simply dropped.
                return_messages.push(first_message);
            }
            // Continue this until there are no more messages, or until the
            // vector contains as many messages as desired by the client.
            // Converting usize to u64 should be safe since we don't consider
            // architectures above 64.
            if return_messages.len() as u64 >= number_of_messages {
                break;
            }
        }

        // Converting usize to u64 should be safe since we don't consider
        // architectures above 64.
        Ok((return_messages, queue.queue.len() as u64))
    }

    /// Load the currently active signing key and the
    /// [`AsIntermediateCredential`].
    async fn load_signing_key(
        &self,
    ) -> Result<AsIntermediateSigningKey, Self::LoadSigningKeyError> {
        self.as_intermediate_signing_key
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)
            .map(|key| key.clone())
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
        let as_credentials = vec![self
            .as_signing_key
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)
            .map(|key| key.credential().clone())?];
        let as_intermediate_credentials = vec![self
            .as_intermediate_signing_key
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)
            .map(|key| key.credential().clone())?];
        Ok((as_credentials, as_intermediate_credentials, vec![]))
    }

    /// Load the OPAQUE [`ServerSetup`].
    async fn load_opaque_setup(
        &self,
    ) -> Result<ServerSetup<OpaqueCiphersuite>, Self::LoadSigningKeyError> {
        self.opaque_server_setup
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)
            .map(|setup| setup.clone())
    }

    // === Anonymous requests ===

    /// Return the client credentials of a user for a given username.
    async fn client_credentials(&self, user_name: &UserName) -> Vec<ClientCredential> {
        let client_records = match self.client_records.read() {
            Ok(records) => records,
            Err(_) => return vec![],
        };
        let mut client_credentials = vec![];
        for (client_id, client_record) in client_records.iter() {
            if client_id.user_name() == *user_name {
                client_credentials.push(client_record.credential.clone());
            }
        }
        client_credentials
    }

    // === PrivacyPass ===

    /// Loads the handle of the PrivacyPass keystore.
    async fn privacy_pass_key_store(&self) -> &Self::PrivacyPassKeyStore {
        &self.privacy_pass_key_store
    }

    /// Loads the number of tokens is still allowed to request.
    async fn load_client_token_allowance(
        &self,
        client_id: &AsClientId,
    ) -> Result<usize, Self::StorageError> {
        let token_allowances = self
            .remaining_tokens
            .read()
            .map_err(|_| AsStorageError::PoisonedLock)?;
        Ok(token_allowances.get(client_id).unwrap_or(&0).to_owned())
    }

    async fn set_client_token_allowance(
        &self,
        client_id: &AsClientId,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError> {
        let mut token_allowances = self
            .remaining_tokens
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?;
        token_allowances.insert(client_id.clone(), number_of_tokens);
        Ok(())
    }

    /// Resets the token allowance of all clients. This should be called after a
    /// rotation of the privacy pass token issuance key material.
    async fn reset_token_allowances(
        &self,
        number_of_tokens: usize,
    ) -> Result<(), Self::StorageError> {
        let mut token_allowances = self
            .remaining_tokens
            .write()
            .map_err(|_| AsStorageError::PoisonedLock)?;
        for (_, allowance) in token_allowances.iter_mut() {
            *allowance = number_of_tokens;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Error)]
pub enum EphemeralStorageError {
    /// The storage is poisoned.
    #[error("The storage is poisoned.")]
    PoisonedLock,
}

#[derive(Debug, Default)]
pub struct EphemeralAsStorage {
    client_credentials: RwLock<HashMap<AsClientId, ClientCredential>>,
    client_login_states:
        RwLock<HashMap<AsClientId, (ClientCredential, ServerLogin<OpaqueCiphersuite>)>>,
    user_login_states: RwLock<HashMap<UserName, ServerLogin<OpaqueCiphersuite>>>,
}

#[async_trait]
impl AsEphemeralStorageProvider for EphemeralAsStorage {
    type StorageError = EphemeralStorageError;

    /// Store a client credential for a given client ID.
    async fn store_credential(
        &self,
        client_id: AsClientId, // TODO: This is probably redundant, as the ID is contained in the credential.
        credential: &ClientCredential,
    ) -> Result<(), Self::StorageError> {
        let mut client_credentials = self
            .client_credentials
            .write()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        client_credentials.insert(client_id, credential.clone());
        Ok(())
    }

    /// Load a client credential for a given client ID.
    async fn load_credential(&self, client_id: &AsClientId) -> Option<ClientCredential> {
        let client_credentials = self.client_credentials.read().ok()?;
        client_credentials.get(client_id).cloned()
    }

    /// Delete a client credential for a given client ID.
    async fn delete_credential(&self, client_id: &AsClientId) -> Result<(), Self::StorageError> {
        let mut client_credentials = self
            .client_credentials
            .write()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        client_credentials.remove(client_id);
        Ok(())
    }

    /// Store the login state for a given client ID.
    async fn store_client_login_state(
        &self,
        client_id: AsClientId, // TODO: This is probably redundant, as the ID is contained in the credential.
        credential: &ClientCredential,
        opaque_state: &ServerLogin<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError> {
        let mut login_states = self
            .client_login_states
            .write()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        login_states.insert(client_id, (credential.clone(), opaque_state.clone()));
        Ok(())
    }

    /// Load the login state for a given client ID.
    async fn load_client_login_state(
        &self,
        client_id: &AsClientId,
    ) -> Result<Option<(ClientCredential, ServerLogin<OpaqueCiphersuite>)>, Self::StorageError>
    {
        let login_states = self
            .client_login_states
            .read()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        Ok(login_states.get(client_id).cloned())
    }

    /// Delete the login state for a given client ID.
    async fn delete_client_login_state(
        &self,
        client_id: &AsClientId,
    ) -> Result<(), Self::StorageError> {
        let mut login_states = self
            .client_login_states
            .write()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        login_states.remove(client_id);
        Ok(())
    }

    /// Store the login state for a given user name.
    async fn store_user_login_state(
        &self,
        user_name: &UserName,
        opaque_state: &ServerLogin<OpaqueCiphersuite>,
    ) -> Result<(), Self::StorageError> {
        let mut login_states = self
            .user_login_states
            .write()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        login_states.insert(user_name.clone(), opaque_state.clone());
        Ok(())
    }

    /// Load the login state for a given user name.
    async fn load_user_login_state(
        &self,
        user_name: &UserName,
    ) -> Result<Option<ServerLogin<OpaqueCiphersuite>>, Self::StorageError> {
        let login_states = self
            .user_login_states
            .read()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        Ok(login_states.get(user_name).cloned())
    }

    /// Delete the login state for a given user name.
    async fn delete_user_login_state(
        &self,
        user_name: &UserName,
    ) -> Result<(), Self::StorageError> {
        let mut login_states = self
            .user_login_states
            .write()
            .map_err(|_| EphemeralStorageError::PoisonedLock)?;
        login_states.remove(user_name);
        Ok(())
    }
}
