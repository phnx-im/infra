use std::{error::Error, fmt::Debug};

use async_trait::async_trait;

use crate::messages::client_qs::EnqueuedMessage;

use super::{client_record::QsClientRecord, user_record::QsUserRecord, ClientId, UserId};

/// Storage provider trait for the QS.
#[async_trait]
pub trait QsStorageProvider: Sync + Send + Debug + 'static {
    type StoreClientError: Error + Debug + PartialEq + Eq + Clone;
    type CreateClientError: Error + Debug + PartialEq + Eq + Clone;
    type DeleteClientError: Error + Debug + PartialEq + Eq + Clone;
    type EnqueueError: Error + Debug + PartialEq + Eq + Clone;
    type ReadAndDeleteError: Error + Debug + PartialEq + Eq + Clone;
    type CreateUserError: Error + Debug + PartialEq + Eq + Clone;
    type StoreUserError: Error + Debug + PartialEq + Eq + Clone;

    /// Stores the client record for a new client and returns the client ID.
    async fn create_client(
        &self,
        client_record: &QsClientRecord,
    ) -> Result<ClientId, Self::CreateClientError>;

    /// Load the info for the client with the given client ID.
    async fn load_client(&self, client_id: &ClientId) -> Option<QsClientRecord>;

    /// Saves a client in the storage provider with the given client ID.
    async fn store_client(
        &self,
        client_id: &ClientId,
        client_record: QsClientRecord,
    ) -> Result<(), Self::StoreClientError>;

    /// Deletes the client with the given clien ID.
    async fn delete_client(&self, client_id: &ClientId) -> Result<(), Self::DeleteClientError>;

    /// Append the given message to the queue. Returns an error if the payload
    /// is greater than the maximum payload allowed by the storage provider.
    async fn enqueue(
        &self,
        client_id: &ClientId,
        message: EnqueuedMessage,
    ) -> Result<(), Self::EnqueueError>;

    /// Delete all messages older than the given sequence number in the queue
    /// with the given id and return up to the requested number of messages from
    /// the queue starting with the message with the given sequence number, as
    /// well as the number of unread messages remaining in the queue.
    async fn read_and_delete(
        &self,
        client_id: &ClientId,
        sequence_number: u64,
        number_of_messages: u64,
    ) -> Result<(Vec<EnqueuedMessage>, u64), Self::ReadAndDeleteError>;

    /// Stores the user record for a new user and returns the user ID.
    async fn create_user(
        &self,
        user_record: &QsUserRecord,
    ) -> Result<UserId, Self::CreateUserError>;

    /// Loads the QsUserRecord for a given QsUid. Returns None if no QsUserRecord
    /// exists for the given QsUid.
    async fn load_user(&self, user_id: &UserId) -> Option<QsUserRecord>;

    /// Stores a QsUserRecord for a given QsUid. If a QsUserRecord already exists
    /// for the given QsUid, it will be overwritten.
    async fn store_user(
        &self,
        user_id: &UserId,
        user_record: QsUserRecord,
    ) -> Result<(), Self::StoreUserError>;

    /// Deletes the QsUserRecord for a given QsUid. Returns true if a QsUserRecord
    /// was deleted, false if no QsUserRecord existed for the given QsUid.
    async fn delete_user_record(&self, user_id: &UserId) -> bool;
}
