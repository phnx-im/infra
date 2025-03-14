// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use super::version::VersionError;

/// Error fetching a message from the QS.
#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsDequeueError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
}

// === Client ===

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsCreateClientRecordError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
    /// Invalid KeyPackage
    #[error("Invalid KeyPackage")]
    InvalidKeyPackage,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsUpdateClientRecordError {
    /// Client not found
    #[error("Client not found")]
    UnknownClient,
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsGetClientError {
    /// Error getting client record
    #[error("Error getting client record")]
    StorageError,
}

// === User ===

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsCreateUserError {
    /// Error creating client record
    #[error("Error creating user record")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsUpdateUserError {
    /// User not found
    #[error("User not found")]
    UnknownUser,
    /// Error updating user record
    #[error("Error updating user record")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsGetUserError {
    /// Error getting user record
    #[error("Error getting user record")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsDeleteUserError {
    /// Error deleteing user record
    #[error("Error deleteing user record")]
    StorageError,
}

// === Key Packages ===

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsPublishKeyPackagesError {
    #[error("Library Error")]
    LibraryError,
    /// Error publishing key packages
    #[error("Error publishing key packages")]
    StorageError,
    /// Invalid KeyPackage
    #[error("Invalid KeyPackage")]
    InvalidKeyPackage,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsClientKeyPackageError {
    /// Error retrieving client key package
    #[error("Error retrieving client key package")]
    StorageError,
    /// No KeyPackages are available
    #[error("No KeyPackages are available")]
    NoKeyPackages,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsKeyPackageError {
    /// Library error
    #[error("Library Error")]
    LibraryError,
    /// Decryption error
    #[error("Decryption error")]
    DecryptionError,
    /// Invalid KeyPackage
    #[error("Invalid KeyPackage")]
    InvalidKeyPackage,
    /// Error retrieving user key packages
    #[error("Error retrieving user key packages")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsEncryptionKeyError {
    /// Library error
    #[error("Library Error")]
    LibraryError,
    /// Error retrieving encryption key
    #[error("Error retrieving encryption key")]
    StorageError,
}

// === Other errors ===

#[derive(Error, Debug)]
pub enum QsProcessError {
    /// Storage Error
    #[error("Storage Error")]
    StorageError,
    /// Authentication error
    #[error("Authentication error")]
    AuthenticationError,
    /// Codec error
    #[error("Codec error")]
    CodecError,
    /// API Version error
    #[error(transparent)]
    Api(#[from] VersionError),

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
    /// Key package error
    #[error("Key package error")]
    QsKeyPackageError(#[from] QsKeyPackageError),

    /// Dequeue error
    #[error("Dequeue error")]
    QsDequeueError(#[from] QsDequeueError),

    /// Encryption key error
    #[error("Encryption key error")]
    QsEncryptionKeyError(#[from] QsEncryptionKeyError),
}
