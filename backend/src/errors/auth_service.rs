// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::time::TimeStamp;
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use tonic::Status;

/// Error fetching a message from the QS.
#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum AsDequeueError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
}

impl From<AsDequeueError> for Status {
    fn from(_e: AsDequeueError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum RegisterUserError {
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

impl From<RegisterUserError> for Status {
    fn from(_e: RegisterUserError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum DeleteUserError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<DeleteUserError> for Status {
    fn from(_e: DeleteUserError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum InitClientAdditionError {
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
pub(crate) enum FinishClientAdditionError {
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
pub(crate) enum DeleteClientError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum PublishConnectionPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Invalid KeyPackage
    #[error("Invalid KeyPackage")]
    InvalidKeyPackage,
}

impl From<PublishConnectionPackageError> for Status {
    fn from(_e: PublishConnectionPackageError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum UserConnectionPackagesError {
    /// User could not be found
    #[error("User could not be found")]
    UnknownUser,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<UserConnectionPackagesError> for Status {
    fn from(_e: UserConnectionPackagesError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum EnqueueMessageError {
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

impl From<EnqueueMessageError> for Status {
    fn from(_e: EnqueueMessageError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum IssueTokensError {
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
pub(crate) enum AsCredentialsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<AsCredentialsError> for Status {
    fn from(_e: AsCredentialsError) -> Self {
        todo!()
    }
}

#[derive(Debug, Error)]
#[repr(u8)]
pub(crate) enum GetUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<GetUserProfileError> for Status {
    fn from(_e: GetUserProfileError) -> Self {
        todo!()
    }
}

#[derive(Error, Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub(crate) enum UpdateUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<UpdateUserProfileError> for Status {
    fn from(_e: UpdateUserProfileError) -> Self {
        todo!()
    }
}
