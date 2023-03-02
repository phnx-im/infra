use super::storage_provider_trait::QsStorageProvider;
use thiserror::Error;

/// Error enqueuing a fanned-out message.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EnqueueFanOutError<S: QsStorageProvider> {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError, // E.g. an error while encoding a message before enqueing it.
    /// Error authenticating the enqueue query
    #[error("Error authenticating the enqueue query")]
    AuthenticationFailure, // E.g. wrong mac
    /// Error enqueuing the message in the underlying queue
    #[error("Error enqueuing the message in the underlying queue")]
    EnqueuingError(EnqueueBasicError<S>),
    /// Error sending push notification.
    #[error("Error sending push notification.")]
    PushNotificationError,
}

/// Error enqueuing direct message
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EnqueueDirectError<S: QsStorageProvider> {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError, // E.g. an error while encoding a message before enqueing it.
    /// Error enqueuing the message in the underlying queue
    #[error("Error enqueuing the message in the underlying queue")]
    EnqueuingError(EnqueueBasicError<S>),
}

/// Error enqueuing a message in the underlying queue
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EnqueueBasicError<S: QsStorageProvider> {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError, // E.g. an error while encoding a message before enqueing it.
    /// Error in the underlying storage provider
    #[error("Error in the underlying storage provider")]
    StorageProviderError(S::EnqueueError),
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
pub enum QsEnqueueError<S: QsStorageProvider> {
    /// The given queue id points to a queue with the wrong type
    #[error("The given queue id points to a queue with the wrong type")]
    WrongQueueType, //
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// An unrecoverable internal error ocurred
    #[error("An unrecoverable internal error ocurred")]
    LibraryError,
    /// An error ocurred enqueueing in a fan out queue
    #[error("An error ocurred enqueueing in a fan out queue")]
    EnqueueFanOutError(EnqueueFanOutError<S>),
    /// An error ocurred enqueueing in a direct queue
    #[error("An error ocurred enqueueing in a direct queue")]
    EnqueueDirectError(EnqueueDirectError<S>),
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

/// Error creating new queue.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsCreateQueueError {
    /// Failed to store queue
    #[error("Failed to store queue")]
    StorageError,
    /// Failed to verify the signature.
    #[error("Failed to verify the signature.")]
    InvalidSignature,
}

/// Error deleting queue.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsDeleteQueueError {
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// Error deleting queue from storage provider
    #[error("Error deleting queue from storage provider")]
    StorageError,
    /// Failed to decrypt authentication key
    #[error("Failed to decrypt authentication key")]
    AuthKeyDecryptionFailure,
    /// Error verifying request authenticity
    #[error("Error verifying request authenticity")]
    AuthenticationFailure,
    /// An unrecoverable internal error ocurred
    #[error("An unrecoverable internal error ocurred")]
    LibraryError,
}
