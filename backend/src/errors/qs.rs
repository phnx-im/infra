// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

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
