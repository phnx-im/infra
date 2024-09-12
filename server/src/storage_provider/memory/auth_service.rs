// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, VecDeque},
    sync::RwLock,
};

use async_trait::async_trait;
use mls_assist::openmls_traits::types::SignatureScheme;
use opaque_ke::{rand::rngs::OsRng, ServerSetup};
use phnxbackend::auth_service::storage_provider_trait::AsStorageProvider;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::OpaqueCiphersuite,
    identifiers::{AsClientId, Fqdn, QualifiedUserName},
    messages::{client_as::ConnectionPackage, QueueMessage},
};
use privacypass_middleware::memory_stores::MemoryKeyStore;
use thiserror::Error;

use super::qs::QueueData;

pub struct MemoryAsStorage {
    connection_packages: RwLock<HashMap<AsClientId, VecDeque<ConnectionPackage>>>,
    queues: RwLock<HashMap<AsClientId, QueueData>>,
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
        _as_domain: Fqdn,
        _signature_scheme: SignatureScheme,
    ) -> Result<Self, StorageInitError> {
        let mut rng = OsRng;
        let opaque_server_setup = RwLock::new(ServerSetup::<OpaqueCiphersuite>::new(&mut rng));
        let privacy_pass_key_store = MemoryKeyStore::default();

        let storage_provider = Self {
            connection_packages: RwLock::new(HashMap::new()),
            queues: RwLock::new(HashMap::new()),
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

const _DEFAULT_NUMBER_OF_TOKENS: usize = 100;

#[async_trait]
impl AsStorageProvider for MemoryAsStorage {
    type PrivacyPassKeyStore = MemoryKeyStore;
    type StorageError = AsStorageError;

    type StoreClientError = AsStorageError;
    type CreateClientError = AsCreateClientError;
    type DeleteClientError = AsStorageError;

    type EnqueueError = AsQueueError;
    type ReadAndDeleteError = ReadAndDeleteError;

    type StoreKeyPackagesError = AsStorageError;
    type LoadConnectionPackageError = AsStorageError;

    type LoadSigningKeyError = AsStorageError;
    type LoadAsCredentialsError = AsStorageError;

    type LoadOpaqueKeyError = AsStorageError;

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
    async fn client_connection_package(
        &self,
        client_id: &AsClientId,
    ) -> Result<ConnectionPackage, Self::LoadConnectionPackageError> {
        let mut connection_package_store = self.connection_packages.write().map_err(|e| {
            tracing::error!("Failed to get connection package store: {:?}", e);
            AsStorageError::PoisonedLock
        })?;

        let Some(connection_packages) = connection_package_store.get_mut(client_id) else {
            tracing::warn!(
                "Did not find connection package for client with id {:?}.",
                client_id
            );
            return Err(AsStorageError::PoisonedLock);
        };
        let result = if connection_packages.len() == 1 {
            connection_packages.front().cloned()
        } else {
            connection_packages.pop_front()
        };

        match result {
            Some(connection_package) => Ok(connection_package),
            None => {
                tracing::warn!(
                    "Did not find connection package for client with id {:?}.",
                    client_id
                );
                Err(AsStorageError::PoisonedLock)
            }
        }
    }

    /// Return a key package for each client of a user referenced by a
    /// user name.
    async fn load_user_connection_packages(
        &self,
        _user_name: &QualifiedUserName,
    ) -> Result<Vec<ConnectionPackage>, Self::StorageError> {
        todo!()
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
    async fn client_credentials(&self, _user_name: &QualifiedUserName) -> Vec<ClientCredential> {
        todo!()
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
