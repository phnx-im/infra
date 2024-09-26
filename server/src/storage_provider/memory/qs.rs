// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, VecDeque},
    sync::RwLock,
};

use async_trait::async_trait;
use phnxtypes::{
    crypto::hpke::ClientIdDecryptionKey,
    identifiers::{Fqdn, QsClientId, QsUserId},
    keypackage_batch::QsEncryptedAddPackage,
    messages::{FriendshipToken, QueueMessage},
};
use thiserror::Error;

use phnxbackend::qs::storage_provider_trait::QsStorageProvider;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

#[derive(Debug)]
pub(super) struct QueueData {
    pub(super) queue: VecDeque<QueueMessage>,
    pub(super) sequence_number: u64,
}

impl QueueData {
    pub(super) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            sequence_number: 0,
        }
    }
}

#[derive(Debug)]
struct KeyPackages {
    key_packages: Vec<QsEncryptedAddPackage>,
    last_resort_key_package: Option<QsEncryptedAddPackage>,
}

impl KeyPackages {
    fn new() -> Self {
        Self {
            key_packages: Vec::new(),
            last_resort_key_package: None,
        }
    }

    fn load_key_package(&mut self) -> Option<QsEncryptedAddPackage> {
        if let Some(key_package) = self.key_packages.pop() {
            Some(key_package)
        } else {
            self.last_resort_key_package.clone()
        }
    }

    fn add_key_packages(&mut self, key_packages: Vec<QsEncryptedAddPackage>) {
        self.key_packages = key_packages;
    }

    fn add_last_resort_key_package(&mut self, key_package: QsEncryptedAddPackage) {
        self.last_resort_key_package = Some(key_package);
    }
}

/// An thread-safe, in-memory implementation of an [`QsStorageProvider`] based
/// on [`HashMap`]s.
#[derive(Debug)]
pub struct MemStorageProvider {
    key_packages: RwLock<HashMap<QsClientId, KeyPackages>>,
    queues: RwLock<HashMap<QsClientId, QueueData>>,
    client_id_decryption_key: ClientIdDecryptionKey,
    domain: Fqdn,
}

impl MemStorageProvider {
    pub fn new(domain: Fqdn) -> Self {
        let config = domain;
        let client_id_decryption_key = ClientIdDecryptionKey::generate().unwrap();
        let key_packages = RwLock::new(HashMap::new());
        let queues = RwLock::new(HashMap::new());
        Self {
            domain: config,
            client_id_decryption_key,
            key_packages,
            queues,
        }
    }
}

#[async_trait]
impl QsStorageProvider for MemStorageProvider {
    type EnqueueError = QueueError;
    type ReadAndDeleteError = ReadAndDeleteError;
    type StoreKeyPackagesError = StoreKeyPackagesError;
    type LoadUserKeyPackagesError = LoadUserKeyPackagesError;

    type LoadSigningKeyError = LoadSigningKeyError;
    type LoadDecryptionKeyError = LoadDecryptionKeyError;

    type LoadConfigError = LoadConfigError;

    async fn own_domain(&self) -> Fqdn {
        self.domain.clone()
    }

    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        let mut key_packages = self
            .key_packages
            .write()
            .map_err(|_| StoreKeyPackagesError::StorageError)?;
        let client_kps = key_packages
            .get_mut(client_id)
            .ok_or(StoreKeyPackagesError::UnknownClient)?;
        client_kps.add_key_packages(encrypted_key_packages);
        Ok(())
    }

    async fn store_last_resort_key_package(
        &self,
        client_id: &QsClientId,
        encrypted_key_package: QsEncryptedAddPackage,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        let mut key_packages = self
            .key_packages
            .write()
            .map_err(|_| StoreKeyPackagesError::StorageError)?;
        let client_kps = key_packages
            .get_mut(client_id)
            .ok_or(StoreKeyPackagesError::UnknownClient)?;
        client_kps.add_last_resort_key_package(encrypted_key_package);
        Ok(())
    }

    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage> {
        let mut key_packages = self.key_packages.write().ok()?;
        let client_key_packages = key_packages.get_mut(client_id)?;
        client_key_packages.load_key_package()
    }

    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Result<Vec<QsEncryptedAddPackage>, LoadUserKeyPackagesError> {
        let mut user_key_packages = vec![];
        let mut key_packages = self.key_packages.write().map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            LoadUserKeyPackagesError::StorageError
        })?;
        Ok(user_key_packages)
    }

    async fn enqueue(
        &self,
        client_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError> {
        let mut queues = self.queues.write().map_err(|_| QueueError::StorageError)?;
        let queue = queues.get_mut(client_id).ok_or(QueueError::QueueNotFound)?;

        // Check if sequence numbers are consistent.
        if queue.sequence_number != message.sequence_number {
            tracing::warn!(
                "Sequence number mismatch. Message sequence number {}, queue sequence number {}",
                message.sequence_number,
                queue.sequence_number
            );
            return Err(QueueError::SequenceNumberMismatch);
        }
        queue.sequence_number += 1;
        queue.queue.push_back(message);
        Ok(())
    }

    async fn read_and_delete(
        &self,
        client_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError> {
        let mut queues = self
            .queues
            .write()
            .map_err(|_| ReadAndDeleteError::StorageError)?;
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
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum StoreUserError {
    /// Cannot access user records.
    #[error("Cannot access user records.")]
    StorageError,
}
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DeleteUserError {
    /// Cannot access user records.
    #[error("Cannot access user records.")]
    StorageError,
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
}
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum StoreClientError {
    /// Cannot access user records.
    #[error("Cannot access user records.")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum CreateClientError {
    /// Cannot access user records.
    #[error("Cannot access user records.")]
    StorageError,
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DeleteClientError {
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
    /// Unknown client.
    #[error("Unknown client.")]
    UnknownClient,
    /// Cannot access user records.
    #[error("Cannot access user records.")]
    StorageError,
}
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum StoreKeyPackagesError {
    /// Cannot access key package store.
    #[error("Cannot access key package store.")]
    StorageError,
    /// Unknown client.
    #[error("Unknown client.")]
    UnknownClient,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum LoadUserKeyPackagesError {
    /// Cannot access key package store.
    #[error("Cannot access key package store.")]
    StorageError,
    /// Unknown user.
    #[error("Unknown user.")]
    UnknownUser,
}

/// Error creating user
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CreateUserError {
    /// Cannot access queue storage.
    #[error("Cannot access queue storage.")]
    StorageError,
}

/// Error creating queue
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CreateQueueError {
    /// The given queue id collides with an existing one.
    #[error("The given queue id collides with an existing one.")]
    QueueIdCollision,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
}

/// General error while accessing the requested queue.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QueueError {
    /// Cannot access queue storage.
    #[error("Cannot access queue storage.")]
    StorageError,
    /// A queue with the given id could not be found.
    #[error("A queue with the given id could not be found.")]
    QueueNotFound,
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
}

/// Error while trying to read and delete messages from queue.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ReadAndDeleteError {
    /// Cannot access queue storage.
    #[error("Cannot access queue storage.")]
    StorageError,
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

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum LoadSigningKeyError {}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum LoadDecryptionKeyError {}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum LoadConfigError {}
