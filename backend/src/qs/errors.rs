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

// === Messages ===

/// Error fetching a message from the QS.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsEnqueueError<S: QsStorageProvider> {
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// Unseal error
    #[error(transparent)]
    UnsealError(#[from] UnsealError),
    /// An error ocurred enqueueing in a fan out queue
    #[error(transparent)]
    EnqueueError(#[from] EnqueueError<S>),
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

// === Client ===

/// Error creating new client.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateClientError<S: QsStorageProvider> {
    /// Failed to store client record
    #[error("Failed to store client record")]
    StorageProviderError(S::CreateClientError),
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateClientRecordError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
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
pub enum QsGetClientError {
    /// Error getting client record
    #[error("Error getting client record")]
    StorageError,
}

// === User ===

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateUserError {
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsUpdateUserError {
    /// Error updating user record
    #[error("Error updating user record")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsGetUserError {
    /// Error getting user record
    #[error("Error getting user record")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsDeleteUserError {
    /// Error deleteing user record
    #[error("Error deleteing user record")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsStoreUserError<S: QsStorageProvider> {
    /// Error creating client record
    #[error("Error creating user record")]
    StorageProviderError(S::StoreUserError),
}

// === Key Packages ===

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsPublishKeyPackagesError {
    #[error("Library Error")]
    LibraryError,
    /// Error publishing key packages
    #[error("Error publishing key packages")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsClientKeyPackageError {
    /// Error retrieving client key package
    #[error("Error retrieving client key package")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsKeyPackageBatchError {
    #[error("Library Error")]
    LibraryError,
    /// Decryption error
    #[error("Decryption error")]
    DecryptionError,
    /// Error retrieving user key packages
    #[error("Error retrieving user key packages")]
    StorageError,
}

// === Local errors ===

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UnsealError {
    /// Decryption error
    #[error("Decryption error")]
    DecryptionError,
    /// Codec error
    #[error("Codec error")]
    CodecError,
}
