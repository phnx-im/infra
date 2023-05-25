// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::KeyPackageVerifyError;
use thiserror::Error;

/// Error fetching a message from the QS.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum AsDequeueError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum InitUserRegistrationError {
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
    #[error("Invalid CSR")]
    InvalidCsr,
    /// Error during OPAQUE registration
    #[error("Error during OPAQUE registration")]
    OpaqueRegistrationFailed,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FinishUserRegistrationError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client credential not found
    #[error("Client credential not found")]
    ClientCredentialNotFound,
    /// Error finishing OPAQUE login handshake
    #[error("Error finishing OPAQUE login handshake")]
    OpaqueLoginFinishFailed,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DeleteUserError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UserClientsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum InitClientAdditionError {
    /// Library error
    #[error("Library error")]
    LibraryError,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client already exists
    #[error("Client already exists")]
    ClientAlreadyExists,
    /// Invalid CSR
    #[error("Invalid CSR")]
    InvalidCsr,
    /// Error during OPAQUE login handshake
    #[error("Error during OPAQUE login handshake")]
    OpaqueLoginFailed,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FinishClientAdditionError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Client credential not found
    #[error("Client credential not found")]
    ClientCredentialNotFound,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DeleteClientError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, Clone)]
pub enum PublishKeyPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Invalid KeyPackage
    #[error(transparent)]
    KeyPackageValidationError(#[from] KeyPackageVerifyError),
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum ClientKeyPackageError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UserKeyPackagesError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
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

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum IssueTokensError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Too many tokens
    #[error("Too many tokens")]
    TooManyTokens,
    /// PrivacyPass protocol error
    #[error("PrivacyPass protocol error")]
    PrivacyPassError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
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

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum AsCredentialsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
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
}

#[derive(Error, Debug, Clone)]
pub enum AsProcessingError {
    /// Authentication error
    #[error(transparent)]
    AuthenticationError(#[from] AsVerificationError),
    #[error(transparent)]
    AsDequeueError(#[from] AsDequeueError),
    #[error(transparent)]
    InitUserRegistrationError(#[from] InitUserRegistrationError),
    #[error(transparent)]
    FinishUserRegistrationError(#[from] FinishUserRegistrationError),
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
    PublishKeyPackageError(#[from] PublishKeyPackageError),
    #[error(transparent)]
    ClientKeyPackageError(#[from] ClientKeyPackageError),
    #[error(transparent)]
    UserKeyPackagesError(#[from] UserKeyPackagesError),
    #[error(transparent)]
    EnqueueMessageError(#[from] EnqueueMessageError),
    #[error(transparent)]
    IssueTokensError(#[from] IssueTokensError),
    #[error(transparent)]
    Init2FactorAuthError(#[from] Init2FactorAuthError),
    #[error(transparent)]
    AsCredentialsError(#[from] AsCredentialsError),
}
