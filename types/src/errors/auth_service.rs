// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::time::TimeStamp;

use super::version::VersionError;

/// Error fetching a message from the QS.
#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum AsDequeueError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum InitUserRegistrationError {
    /// Could not find signing key
    #[error("Could not find signing key")]
    SigningKeyNotFound,
    /// Library error
    #[error("Library error")]
    LibraryError,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// User already exists
    #[error("User already exists")]
    UserAlreadyExists,
    /// Invalid CSR
    #[error("Invalid CSR: Time now: {0:?}, not valid before: {1:?}, not valid after: {2:?}")]
    InvalidCsr(TimeStamp, TimeStamp, TimeStamp),
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum DeleteUserError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum UserClientsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum InitClientAdditionError {
    /// Library error
    #[error("Library error")]
    LibraryError,
    /// Could not find signing key
    #[error("Could not find signing key")]
    SigningKeyNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client already exists
    #[error("Client already exists")]
    ClientAlreadyExists,
    /// Invalid CSR
    #[error("Invalid CSR: Time now: {0:?}, not valid before: {1:?}, not valid after: {2:?}")]
    InvalidCsr(TimeStamp, TimeStamp, TimeStamp),
    /// Error during OPAQUE login handshake
    #[error("Error during OPAQUE login handshake")]
    OpaqueLoginFailed,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum FinishClientAdditionError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client credential not found
    #[error("Client credential not found")]
    ClientCredentialNotFound,
    /// Invalid connection package
    #[error("Invalid connection package")]
    InvalidConnectionPackage,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum DeleteClientError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum PublishConnectionPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Invalid KeyPackage
    #[error("Invalid KeyPackage")]
    InvalidKeyPackage,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum ClientKeyPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum UserConnectionPackagesError {
    /// User could not be found
    #[error("User could not be found")]
    UnknownUser,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum EnqueueMessageError {
    /// Library error
    #[error("Library error")]
    LibraryError,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client not found
    #[error("Client not found")]
    ClientNotFound,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum IssueTokensError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Too many tokens
    #[error("Too many tokens")]
    TooManyTokens,
    /// Unknown client
    #[error("Unknown client")]
    UnknownClient,
    /// PrivacyPass protocol error
    #[error("PrivacyPass protocol error")]
    PrivacyPassError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum Init2FactorAuthError {
    /// Library error
    #[error("Library error")]
    LibraryError,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Error during OPAQUE login handshake
    #[error("Error during OPAQUE login handshake")]
    OpaqueLoginFailed,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum AsCredentialsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug)]
#[repr(u8)]
pub enum AsVerificationError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Could not find client
    #[error("Could not find client")]
    UnknownClient,
    /// Could not find user
    #[error("Could not find user")]
    UnknownUser,
    /// Could not authenticate message
    #[error("Could not authenticate message")]
    AuthenticationFailed,
    /// API Version error
    #[error(transparent)]
    Api(#[from] VersionError),
}

#[derive(Debug, Error)]
#[repr(u8)]
pub enum GetUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum UpdateUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Debug, Error)]
#[repr(u8)]
pub enum AsProcessingError {
    /// API Version error
    #[error(transparent)]
    Api(#[from] VersionError),
    /// Authentication error
    #[error(transparent)]
    AuthenticationError(#[from] AsVerificationError),
    #[error(transparent)]
    AsDequeueError(#[from] AsDequeueError),
    #[error(transparent)]
    InitUserRegistrationError(#[from] InitUserRegistrationError),
    #[error(transparent)]
    DeleteUserError(#[from] DeleteUserError),
    #[error(transparent)]
    UserClientsError(#[from] UserClientsError),
    #[error(transparent)]
    InitClientAdditionError(#[from] InitClientAdditionError),
    #[error(transparent)]
    FinishClientAdditionError(#[from] FinishClientAdditionError),
    #[error(transparent)]
    DeleteClientError(#[from] DeleteClientError),
    #[error(transparent)]
    PublishKeyPackageError(#[from] PublishConnectionPackageError),
    #[error(transparent)]
    ClientKeyPackageError(#[from] ClientKeyPackageError),
    #[error(transparent)]
    UserKeyPackagesError(#[from] UserConnectionPackagesError),
    #[error(transparent)]
    EnqueueMessageError(#[from] EnqueueMessageError),
    #[error(transparent)]
    IssueTokensError(#[from] IssueTokensError),
    #[error(transparent)]
    Init2FactorAuthError(#[from] Init2FactorAuthError),
    #[error(transparent)]
    AsCredentialsError(#[from] AsCredentialsError),
    #[error(transparent)]
    GetUserProfileError(#[from] GetUserProfileError),
    #[error(transparent)]
    UpdateUserProfileError(#[from] UpdateUserProfileError),
}
