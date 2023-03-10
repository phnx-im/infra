use std::{error::Error, fmt::Debug};

use async_trait::async_trait;

use crate::messages::{client_qs::QueueMessage, FriendshipToken};

use super::{
    client_record::QsClientRecord, user_record::QsUserRecord, QsClientId, QsEncryptedAddPackage,
    QsUserId,
};

/// Storage provider trait for the QS.
#[async_trait]
pub trait QsStorageProvider: Sync + Send + Debug + 'static {
    type CreateUserError: Error + Debug + PartialEq + Eq + Clone;
    type StoreUserError: Error + Debug + PartialEq + Eq + Clone;
    type DeleteUserError: Error + Debug + PartialEq + Eq + Clone;

    type StoreClientError: Error + Debug + PartialEq + Eq + Clone;
    type CreateClientError: Error + Debug + PartialEq + Eq + Clone;
    type DeleteClientError: Error + Debug + PartialEq + Eq + Clone;

    type EnqueueError: Error + Debug + PartialEq + Eq + Clone;
    type ReadAndDeleteError: Error + Debug + PartialEq + Eq + Clone;

    type StoreKeyPackagesError: Error + Debug + PartialEq + Eq + Clone;

    // === USERS ===

    /// Returns a new unique user ID.
    async fn create_user(&self) -> Result<QsUserId, Self::CreateUserError>;

    /// Loads the QsUserRecord for a given UserId. Returns None if no QsUserRecord
    /// exists for the given UserId.
    async fn load_user(&self, user_id: &QsUserId) -> Option<QsUserRecord>;

    /// Stores a QsUserRecord for a given UserId. If a QsUserRecord already exists
    /// for the given UserId, it will be overwritten.
    async fn store_user(
        &self,
        user_id: &QsUserId,
        user_record: QsUserRecord,
    ) -> Result<(), Self::StoreUserError>;

    /// Deletes the QsUserRecord for a given UserId. Returns true if a QsUserRecord
    /// was deleted, false if no QsUserRecord existed for the given UserId.
    ///
    /// The storage provider must also delete the following:
    ///  - All clients of the user
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_user(&self, user_id: &QsUserId) -> Result<(), Self::DeleteUserError>;

    // === CLIENTS ===

    /// Returns a new unique client ID.
    async fn create_client(&self) -> Result<QsClientId, Self::CreateClientError>;

    /// Load the info for the client with the given client ID.
    async fn load_client(&self, client_id: &QsClientId) -> Option<QsClientRecord>;

    /// Saves a client in the storage provider with the given client ID. The
    /// storage provider must associate this client with the user of the client.
    async fn store_client(
        &self,
        client_id: &QsClientId,
        client_record: QsClientRecord,
    ) -> Result<(), Self::StoreClientError>;

    /// Deletes the client with the given client ID.
    ///
    /// The storage provider must also delete the following:
    ///  - The associated user, if the user has no other clients
    ///  - All enqueued messages for the respective clients
    ///  - All key packages for the respective clients
    async fn delete_client(&self, client_id: &QsClientId) -> Result<(), Self::DeleteClientError>;

    // === KEY PACKAGES ===

    /// Store key packages for a specific client.
    async fn store_key_packages(
        &self,
        client_id: &QsClientId,
        encrypted_key_packages: Vec<QsEncryptedAddPackage>,
    ) -> Result<(), Self::StoreKeyPackagesError>;

    /// Return a key package for a specific client. The user ID is used to check if
    /// the client belongs to the user.
    /// TODO: Last resort key package
    async fn load_key_package(
        &self,
        user_id: &QsUserId,
        client_id: &QsClientId,
    ) -> Option<QsEncryptedAddPackage>;

    /// Return a key package for each client of a user ereferenced by a
    /// friendship token.
    async fn load_user_key_packages(
        &self,
        friendship_token: &FriendshipToken,
    ) -> Vec<QsEncryptedAddPackage>;

    // === MESSAGES ===

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    async fn enqueue(
        &self,
        client_id: &QsClientId,
        message: QueueMessage,
    ) -> Result<(), Self::EnqueueError>;

    /// Delete all messages older than the given sequence number in the queue
    /// with the given client ID and return up to the requested number of
    /// messages from the queue starting with the message with the given
    /// sequence number, as well as the number of unread messages remaining in
    /// the queue.
    async fn read_and_delete(
        &self,
        client_id: &QsClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<QueueMessage>, u64), Self::ReadAndDeleteError>;
}
