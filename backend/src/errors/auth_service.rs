// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcommon::time::TimeStamp;
use thiserror::Error;
use tonic::Status;
use tracing::error;

use super::StorageError;

#[derive(Error, Debug)]
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
pub(crate) enum DeleteUserError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<sqlx::Error> for DeleteUserError {
    fn from(e: sqlx::Error) -> Self {
        error!(%e, "Error deleting user");
        DeleteUserError::StorageError
    }
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
pub(crate) enum IssueTokensError {
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Too many tokens
    #[error("Too many tokens")]
    TooManyTokens,
    /// Unknown user
    #[error("Unknown user")]
    UnknownUser,
    /// PrivacyPass protocol error
    #[error("PrivacyPass protocol error")]
    PrivacyPassError,
}

impl From<IssueTokensError> for Status {
    fn from(e: IssueTokensError) -> Self {
        let msg = e.to_string();
        match e {
            IssueTokensError::StorageError => Status::internal(msg),
            IssueTokensError::TooManyTokens => Status::resource_exhausted(msg),
            IssueTokensError::UnknownUser => Status::internal(msg),
            IssueTokensError::PrivacyPassError => Status::internal(msg),
        }
    }
}

#[derive(Error, Debug)]
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
pub(crate) enum GetUserProfileError {
    #[error("No ciphertext matching index")]
    NoCiphertextFound,
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<StorageError> for GetUserProfileError {
    fn from(error: StorageError) -> Self {
        error!(%error, "Error loading user record");
        Self::StorageError
    }
}

impl From<GetUserProfileError> for Status {
    fn from(e: GetUserProfileError) -> Self {
        let msg = e.to_string();
        match e {
            GetUserProfileError::NoCiphertextFound => Status::invalid_argument(msg),
            GetUserProfileError::UserNotFound => Status::not_found(msg),
            GetUserProfileError::StorageError => Status::internal(msg),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum StageUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
}

impl From<StorageError> for StageUserProfileError {
    fn from(error: StorageError) -> Self {
        error!(%error, "Error loading user record");
        Self::StorageError
    }
}

impl From<StageUserProfileError> for Status {
    fn from(e: StageUserProfileError) -> Self {
        let msg = e.to_string();
        match e {
            StageUserProfileError::UserNotFound => Status::not_found(msg),
            StageUserProfileError::StorageError => Status::internal(msg),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum MergeUserProfileError {
    #[error("User not found")]
    UserNotFound,
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// No staged user profile
    #[error("No staged user profile")]
    NoStagedUserProfile,
}

impl From<StorageError> for MergeUserProfileError {
    fn from(error: StorageError) -> Self {
        error!(%error, "Error loading user record");
        Self::StorageError
    }
}

impl From<MergeUserProfileError> for Status {
    fn from(e: MergeUserProfileError) -> Self {
        let msg = e.to_string();
        match e {
            MergeUserProfileError::UserNotFound => Status::not_found(msg),
            MergeUserProfileError::StorageError => Status::internal(msg),
            MergeUserProfileError::NoStagedUserProfile => Status::not_found(msg),
        }
    }
}
