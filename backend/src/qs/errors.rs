use super::storage_provider_trait::QsStorageProvider;
use thiserror::Error;

/// Error enqueuing a fanned-out message.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EnqueueError<S: QsStorageProvider> {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError, // E.g. an error while encoding a message before enqueing it.
    /// Error in the underlying storage provider
    #[error("Error in the underlying storage provider")]
    StorageProviderError(S::EnqueueError),
    /// Error sending push notification.
    #[error("Error sending push notification.")]
    PushNotificationError,
}

/// Error authenticating a request
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum RequestAuthenticationError {
    /// Error decrypting the authentication key
    #[error("Error decrypting the authentication key")]
    AuthKeyDecryptionFailure, // E.g. an error while encoding a message before enqueing it.
    /// Error authenticating the request
    #[error("Error authenticating the request")]
    AuthenticationError,
}

/// Error fetching a message from the QS.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsEnqueueProviderError {
    /// An unrecoverable internal error ocurred
    #[error("An unrecoverable internal error ocurred")]
    LibraryError,
}

/// Error fetching a message from the QS.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsEnqueueError<S: QsStorageProvider> {
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// An error ocurred enqueueing in a fan out queue
    #[error("An error ocurred enqueueing in a fan out queue")]
    EnqueueError(EnqueueError<S>),
}

/// Error fetching a message from the QS.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsFetchError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// Invalid signature
    #[error("Invalid signature")]
    InvalidSignature,
}

/// Error updating queue info.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsUpdateQueueError {
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// Unrecoverable server error
    #[error("Internal Server Error")]
    StorageError,
    /// The requested queue and the given queue info don't match.
    #[error("The requested queue and the given queue info don't match")]
    WrongQueueType,
}

/// Error creating new client.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateClientError<S: QsStorageProvider> {
    /// Failed to store client record
    #[error("Failed to store client record")]
    StorageProviderError(S::CreateClientError),
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateClientRecordError {
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsUpdateClientRecordError {
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateUserError<S: QsStorageProvider> {
    /// Error updating client record
    #[error("Error updating client record")]
    ClientCreationError(#[from] QsCreateClientError<S>),
    /// Failed to store user record
    #[error("Failed to store client record")]
    StorageProviderError(S::CreateUserError),
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsUpdateUserError {
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
}
