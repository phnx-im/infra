// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::time::TimeStamp;
use thiserror::Error;
use tonic::Status;

/// Error fetching a message from the QS.
#[derive(Error, Debug)]
#[repr(u8)]
pub(crate) enum AsDequeueError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<AsDequeueError> for Status {
    fn from(e: AsDequeueError) -> Self {
        let msg = e.to_string();
        match e {
            AsDequeueError::StorageError => Status::internal(msg),
        }
    }
}

#[derive(Error, Debug)]
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
    fn from(e: RegisterUserError) -> Self {
        let msg = e.to_string();
        match e {
            RegisterUserError::SigningKeyNotFound => Status::not_found(msg),
            RegisterUserError::LibraryError | RegisterUserError::StorageError => {
                Status::internal(msg)
            }
            RegisterUserError::UserAlreadyExists => Status::already_exists(msg),
            RegisterUserError::InvalidCsr(..) => Status::invalid_argument(msg),
        }
    }
}

#[derive(Error, Debug)]
#[repr(u8)]
pub(crate) enum DeleteUserError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<DeleteUserError> for Status {
    fn from(e: DeleteUserError) -> Self {
        let msg = e.to_string();
        match e {
            DeleteUserError::StorageError => Status::internal(msg),
        }
    }
}

#[derive(Error, Debug)]
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
    fn from(e: PublishConnectionPackageError) -> Self {
        let msg = e.to_string();
        match e {
            PublishConnectionPackageError::StorageError => Status::internal(msg),
            PublishConnectionPackageError::InvalidKeyPackage => Status::invalid_argument(msg),
        }
    }
}

#[derive(Error, Debug)]
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
    fn from(e: UserConnectionPackagesError) -> Self {
        let msg = e.to_string();
        match e {
            UserConnectionPackagesError::UnknownUser => Status::not_found(msg),
            UserConnectionPackagesError::StorageError => Status::internal(msg),
        }
    }
}

#[derive(Error, Debug)]
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
    fn from(e: EnqueueMessageError) -> Self {
        let msg = e.to_string();
        match e {
            EnqueueMessageError::StorageError | EnqueueMessageError::LibraryError => {
                Status::internal(msg)
            }
            EnqueueMessageError::ClientNotFound => Status::not_found(msg),
        }
    }
}

#[derive(Error, Debug)]
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

#[derive(Error, Debug)]
#[repr(u8)]
pub(crate) enum AsCredentialsError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<AsCredentialsError> for Status {
    fn from(e: AsCredentialsError) -> Self {
        let msg = e.to_string();
        match e {
            AsCredentialsError::StorageError => Status::internal(msg),
        }
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
    fn from(e: GetUserProfileError) -> Self {
        let msg = e.to_string();
        match e {
            GetUserProfileError::UserNotFound => Status::not_found(msg),
            GetUserProfileError::StorageError => Status::internal(msg),
        }
    }
}

#[derive(Error, Debug)]
#[repr(u8)]
pub(crate) enum UpdateUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<UpdateUserProfileError> for Status {
    fn from(e: UpdateUserProfileError) -> Self {
        let msg = e.to_string();
        match e {
            UpdateUserProfileError::UserNotFound => Status::not_found(msg),
            UpdateUserProfileError::StorageError => Status::internal(msg),
        }
    }
}
