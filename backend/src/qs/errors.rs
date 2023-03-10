// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::storage_provider_trait::QsStorageProvider;
use thiserror::Error;

// === DS API errors ===

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
pub enum QsDequeueError {
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

// === Other errors ===

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UnsealError {
    /// Decryption error
    #[error("Decryption error")]
    DecryptionError,
    /// Codec error
    #[error("Codec error")]
    CodecError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum SealError {
    /// Encryption error
    #[error("Encryption error")]
    EncryptionError,
    /// Codec error
    #[error("Codec error")]
    CodecError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum QsProcessError {
    /// Authentication error
    #[error("Authentication error")]
    AuthenticationError,
    /// Codec error
    #[error("Codec error")]
    CodecError,

    /// Create user error
    #[error("Create user error")]
    QsCreateUserError(#[from] QsCreateUserError),
    /// Update user error
    #[error("Update user error")]
    QsUpdateUserError(#[from] QsUpdateUserError),
    /// Get user error
    #[error("Get user error")]
    QsGetUserError(#[from] QsGetUserError),
    /// Delete user error
    #[error("Delete user error")]
    QsDeleteUserError(#[from] QsDeleteUserError),

    /// Create client error
    #[error("Create client error")]
    QsCreateClientRecordError(#[from] QsCreateClientRecordError),
    /// Update client error
    #[error("Update client error")]
    QsUpdateClientRecordError(#[from] QsUpdateClientRecordError),
    /// Get client error
    #[error("Get client error")]
    QsGetClientError(#[from] QsGetClientError),

    /// Publish key packages error
    #[error("Publish key packages error")]
    QsPublishKeyPackagesError(#[from] QsPublishKeyPackagesError),
    /// Client key package error
    #[error("Client key package error")]
    QsClientKeyPackageError(#[from] QsClientKeyPackageError),
    /// Key package batch error
    #[error("Key package batch error")]
    QsKeyPackageBatchError(#[from] QsKeyPackageBatchError),

    /// Dequeue error
    #[error("Dequeue error")]
    QsDequeueError(#[from] QsDequeueError),
}
