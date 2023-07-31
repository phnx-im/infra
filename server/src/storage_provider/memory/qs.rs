// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, VecDeque},
    sync::RwLock,
};

use async_trait::async_trait;
use thiserror::Error;

use phnxbackend::{
    messages::{FriendshipToken, QueueMessage},
    qs::{
        client_record::QsClientRecord, storage_provider_trait::QsStorageProvider,
        user_record::QsUserRecord, ClientIdDecryptionKey, Fqdn, QsClientId, QsConfig,
        QsEncryptedAddPackage, QsSigningKey, QsUserId,
    },
};
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

/// An thread-safe, in-memory implementation of an [`QsStorageProvider`] based
/// on [`HashMap`]s.
#[derive(Debug)]
pub struct MemStorageProvider {
    users: RwLock<HashMap<QsUserId, QsUserRecord>>,
    clients: RwLock<HashMap<QsClientId, QsClientRecord>>,
    key_packages: RwLock<HashMap<QsClientId, Vec<QsEncryptedAddPackage>>>,
    queues: RwLock<HashMap<QsClientId, QueueData>>,
    signing_key: QsSigningKey,
    client_id_decryption_key: ClientIdDecryptionKey,
    config: QsConfig,
}

impl MemStorageProvider {
    pub fn new(domain: Fqdn) -> Self {
        let config = QsConfig { domain };
        let client_id_decryption_key = ClientIdDecryptionKey::generate().unwrap();
        let signing_key = QsSigningKey::generate().unwrap();
        let users = RwLock::new(HashMap::new());
        let key_packages = RwLock::new(HashMap::new());
        let clients = RwLock::new(HashMap::new());
        let queues = RwLock::new(HashMap::new());
        Self {
            config,
            client_id_decryption_key,
            signing_key,
            users,
            clients,
            key_packages,
            queues,
        }
    }
}

#[async_trait]
impl QsStorageProvider for MemStorageProvider {
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
        self.config.domain.clone()
    }

    async fn create_user(
        &self,
        user_record: QsUserRecord,
    ) -> Result<QsUserId, Self::CreateUserError> {
        let user_id = QsUserId::random();
        if let Ok(mut users) = self.users.write() {
            users.insert(user_id.clone(), user_record);
            Ok(user_id)
        } else {
            Err(CreateUserError::StorageError)
        }
    }

    async fn load_user(&self, user_id: &QsUserId) -> Option<QsUserRecord> {
        if let Ok(users) = self.users.read() {
            users.get(user_id).cloned()
        } else {
            None
        }
    }

    async fn store_user(
        &self,
        user_id: &QsUserId,
        user_record: QsUserRecord,
    ) -> Result<(), Self::StoreUserError> {
        if let Ok(mut users) = self.users.write() {
            users.insert(user_id.clone(), user_record);
            Ok(())
        } else {
            Err(StoreUserError::StorageError)
        }
    }

    async fn delete_user(&self, user_id: &QsUserId) -> Result<(), Self::DeleteUserError> {
        // Get all locks.
        let mut users = self
            .users
            .write()
            .map_err(|_| DeleteUserError::StorageError)?;
        let mut clients = self
            .clients
            .write()
            .map_err(|_| DeleteUserError::StorageError)?;
        let mut key_packages = self
            .key_packages
            .write()
            .map_err(|_| DeleteUserError::StorageError)?;
        let mut queues = self
            .queues
            .write()
            .map_err(|_| DeleteUserError::StorageError)?;
        // Delete the user
        let user = users.remove(user_id).ok_or(DeleteUserError::UnknownUser)?;
        // Delete all KeyPackages and clients
        for client in user.clients() {
            key_packages.remove(client);
            clients.remove(client);
            queues.remove(client);
        }
        Ok(())
    }

    async fn create_client(
        &self,
        client_record: QsClientRecord,
    ) -> Result<QsClientId, Self::CreateClientError> {
        // TODO: For now, we trust the RNG to prevent collisions.
        let mut users = self
            .users
            .write()
            .map_err(|_| CreateClientError::StorageError)?;
        let mut clients = self
            .clients
            .write()
            .map_err(|_| CreateClientError::StorageError)?;
        let mut key_packages = self
            .key_packages
            .write()
            .map_err(|_| CreateClientError::StorageError)?;
        let mut queues = self
            .queues
            .write()
            .map_err(|_| CreateClientError::StorageError)?;
        let user_record = users
            .get_mut(&client_record.user_id)
            .ok_or(CreateClientError::UnknownUser)?;
        let client_id = QsClientId::random();
        user_record.clients_mut().push(client_id.clone());
        key_packages.insert(client_id.clone(), vec![]);
        clients.insert(client_id.clone(), client_record);
        queues.insert(client_id.clone(), QueueData::new());

        Ok(client_id)
    }

    async fn load_client(&self, client_id: &QsClientId) -> Option<QsClientRecord> {
        if let Ok(clients) = self.clients.read() {
            clients.get(client_id).cloned()
        } else {
            None
        }
    }

    async fn store_client(
        &self,
        client_id: &QsClientId,
        client_record: QsClientRecord,
    ) -> Result<(), Self::StoreClientError> {
        if let Ok(mut clients) = self.clients.write() {
            clients.insert(client_id.clone(), client_record);
            Ok(())
        } else {
            Err(StoreClientError::StorageError)
        }
    }

    async fn delete_client(&self, client_id: &QsClientId) -> Result<(), Self::DeleteClientError> {
        // Get all locks.
        let mut users = self
            .users
            .write()
            .map_err(|_| DeleteClientError::StorageError)?;
        let mut clients = self
            .clients
            .write()
            .map_err(|_| DeleteClientError::StorageError)?;
        let mut key_packages = self
            .key_packages
            .write()
            .map_err(|_| DeleteClientError::StorageError)?;
        let mut queues = self
            .queues
            .write()
            .map_err(|_| DeleteClientError::StorageError)?;
        // Delete the client record.
        let client_record = clients
            .remove(client_id)
            .ok_or(DeleteClientError::UnknownClient)?;
        key_packages.remove(client_id);
        clients.remove(client_id);
        queues.remove(client_id);
        // Delete the client in the user record.
        let user_id = client_record.user_id;
        let user = users
            .get_mut(&user_id)
            .ok_or(DeleteClientError::UnknownUser)?;
        let user_clients = user.clients_mut();
        if let Some(position) = user_clients
            .iter()
            .position(|user_client_id| user_client_id == client_id)
        {
            user_clients.remove(position);
        } else {
            return Err(DeleteClientError::StorageError);
        }
        if user_clients.is_empty() {
            users.remove(&user_id);
        }
        Ok(())
    }

    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        mut encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError> {
        let mut key_packages = self
            .key_packages
            .write()
            .map_err(|_| StoreKeyPackagesError::StorageError)?;
        let client_kps = key_packages
            .get_mut(client_id)
            .ok_or(StoreKeyPackagesError::UnknownClient)?;
        client_kps.append(&mut encrypted_key_packages);
        Ok(())
    }

    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage> {
        let clients = self.clients.read().ok()?;
        let client = clients.get(client_id)?;
        if &client.user_id != user_id {
            return None;
        }
        let mut key_packages = self.key_packages.write().ok()?;
        let client_key_packages = key_packages.get_mut(client_id)?;
        // Workaround for last-resort key packages. If there's only one left,
        // clone it, otherwise pop it.
        if client_key_packages.len() == 1 {
            client_key_packages.first().cloned()
        } else {
            client_key_packages.pop()
        }
    }

    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Vec<QsEncryptedAddPackage> {
        let users = if let Ok(users) = self.users.read() {
            users
        } else {
            return vec![];
        };
        let user = if let Some((_id, record)) = users
            .iter()
            .find(|(_user_id, user_record)| user_record.friendship_token() == friendship_token)
        {
            record
        } else {
            return vec![];
        };
        let mut user_key_packages = vec![];
        let mut key_packages = if let Ok(key_packages) = self.key_packages.write() {
            key_packages
        } else {
            return vec![];
        };
        for client in user.clients() {
            if let Some(client_key_packages) = key_packages.get_mut(client) {
                let client_key_package = if client_key_packages.len() == 1 {
                    client_key_packages.first().cloned()
                } else {
                    client_key_packages.pop()
                }
                // We unwrap here for now, because there should always be one key package.
                .unwrap();
                user_key_packages.push(client_key_package);
            } else {
                // If there is an inconsistency between client and user
                // record, we return an empty vector.
                return vec![];
            }
        }
        user_key_packages
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

    async fn load_signing_key(&self) -> Result<QsSigningKey, Self::LoadSigningKeyError> {
        Ok(self.signing_key.clone())
    }

    async fn load_decryption_key(
        &self,
    ) -> Result<ClientIdDecryptionKey, Self::LoadDecryptionKeyError> {
        Ok(self.client_id_decryption_key.clone())
    }

    async fn load_config(&self) -> Result<QsConfig, Self::LoadConfigError> {
        Ok(self.config.clone())
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
