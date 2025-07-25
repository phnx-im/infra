// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group, memory_provider::MlsAssistMemoryStorage, openmls::group::MergeCommitError,
};
use thiserror::Error;
use tonic::Status;
use tracing::error;

use phnxcommon::codec::PhnxCodec;

pub(crate) mod auth_service;
pub(crate) mod qs;

pub(crate) type CborMlsAssistStorage = MlsAssistMemoryStorage<PhnxCodec>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxcommon::codec::Error),
}

impl From<sqlx::Error> for StorageError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.into())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for StorageError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Database(e.into())
    }
}

impl From<StorageError> for Status {
    fn from(error: StorageError) -> Self {
        error!(%error, "storage error");
        match error {
            StorageError::Database(_) => Self::internal("Database error"),
            StorageError::Serde(_) => Self::internal("Seriazation error"),
        }
    }
}
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Dynamic(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// General error while accessing the requested queue.
#[derive(Error, Debug)]
pub(super) enum QueueError {
    #[error("Database error")]
    Storage(#[from] StorageError),
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers")]
    SequenceNumberMismatch,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
}

impl From<sqlx::Error> for QueueError {
    fn from(e: sqlx::Error) -> Self {
        Self::Storage(e.into())
    }
}

impl From<phnxcommon::codec::Error> for QueueError {
    fn from(e: phnxcommon::codec::Error) -> Self {
        Self::Storage(e.into())
    }
}

impl From<QueueError> for Status {
    fn from(error: QueueError) -> Self {
        let msg = error.to_string();
        match error {
            QueueError::Storage(error) => {
                error!(%error, "storage error");
                Self::internal(msg)
            }
            QueueError::SequenceNumberMismatch | QueueError::LibraryError => Self::internal(msg),
        }
    }
}

/// Potential errors when performing a group operation.
#[derive(Debug, Error)]
pub(crate) enum GroupOperationError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message")]
    ProcessingError,
    /// Missing queue config in client key package.
    #[error("Missing queue config in client key package")]
    MissingQueueConfig,
    /// Incomplete Welcome message.
    #[error("Incomplete Welcome message.")]
    IncompleteWelcome,
    #[error("Error merging commit")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

impl From<GroupOperationError> for Status {
    fn from(e: GroupOperationError) -> Self {
        let msg = e.to_string();
        match e {
            GroupOperationError::LibraryError
            | GroupOperationError::ProcessingError
            | GroupOperationError::InvalidMessage => Status::internal(msg),
            GroupOperationError::MissingQueueConfig | GroupOperationError::IncompleteWelcome => {
                Status::invalid_argument(msg)
            }
            GroupOperationError::MergeCommitError(merge_commit_error) => {
                error!(%merge_commit_error, "failed merging commit");
                Status::internal(msg)
            }
        }
    }
}

/// Potential errors when updating a client.
#[derive(Debug, Error)]
pub(crate) enum ClientUpdateError {
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

impl From<ClientUpdateError> for Status {
    fn from(e: ClientUpdateError) -> Self {
        let msg = e.to_string();
        match e {
            ClientUpdateError::InvalidMessage => Status::invalid_argument(msg),
            ClientUpdateError::ProcessingError => Status::internal(msg),
            ClientUpdateError::MergeCommitError(merge_commit_error) => {
                error!(%merge_commit_error, "failed merging commit");
                Status::internal(msg)
            }
        }
    }
}

/// Potential errors when joining a connection group.
#[derive(Debug, Error)]
pub(crate) enum JoinConnectionGroupError {
    /// Invalid assisted message.
    #[error("Invalid assisted message")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message")]
    ProcessingError,
    /// Not a connection group.
    #[error("Not a connection group")]
    NotAConnectionGroup,
    #[error("Error merging commit")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

impl From<JoinConnectionGroupError> for Status {
    fn from(e: JoinConnectionGroupError) -> Self {
        let msg = e.to_string();
        match e {
            JoinConnectionGroupError::InvalidMessage
            | JoinConnectionGroupError::NotAConnectionGroup => Status::invalid_argument(msg),
            JoinConnectionGroupError::ProcessingError => Status::internal(msg),
            JoinConnectionGroupError::MergeCommitError(merge_commit_error) => {
                error!(%merge_commit_error, "failed merging commit");
                Status::internal(msg)
            }
        }
    }
}

/// Potential errors when adding a user.
#[derive(Debug, Error)]
pub(crate) enum ClientAdditionError {
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when removing clients.
#[derive(Debug, Error)]
pub(crate) enum ClientRemovalError {
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when deleting a group.
#[derive(Debug, Error)]
pub(crate) enum GroupDeletionError {
    /// Invalid assisted message.
    #[error("Invalid assisted message")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message")]
    ProcessingError,
    #[error("Error merging commit")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

impl From<GroupDeletionError> for Status {
    fn from(e: GroupDeletionError) -> Self {
        let msg = e.to_string();
        match e {
            GroupDeletionError::InvalidMessage => Status::invalid_argument(msg),
            GroupDeletionError::ProcessingError => Status::internal(msg),
            GroupDeletionError::MergeCommitError(merge_commit_error) => {
                error!(%merge_commit_error, "failed merging commit");
                Status::internal(msg)
            }
        }
    }
}

/// Potential errors when processing a self remove proposal.
#[derive(Debug, Error)]
pub(crate) enum ClientSelfRemovalError {
    /// Invalid assisted message.
    #[error("Invalid assisted message")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message")]
    ProcessingError,
    #[error("Error merging commit")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

impl From<ClientSelfRemovalError> for Status {
    fn from(e: ClientSelfRemovalError) -> Self {
        let msg = e.to_string();
        match e {
            ClientSelfRemovalError::InvalidMessage => Status::invalid_argument(msg),
            ClientSelfRemovalError::ProcessingError => Status::internal(msg),
            ClientSelfRemovalError::MergeCommitError(merge_commit_error) => {
                error!(%merge_commit_error, "failed merging commit");
                Status::internal(msg)
            }
        }
    }
}

/// Potential errors when resyncing a client.
#[derive(Debug, Error)]
pub(crate) enum ResyncClientError {
    /// Invalid assisted message.
    #[error("Invalid assisted message")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message")]
    ProcessingError,
    #[error("Error merging commit")]
    MergeCommitError(#[from] MergeCommitError<group::errors::StorageError<CborMlsAssistStorage>>),
}

impl From<ResyncClientError> for Status {
    fn from(e: ResyncClientError) -> Self {
        let msg = e.to_string();
        match e {
            ResyncClientError::InvalidMessage => Status::invalid_argument(msg),
            ResyncClientError::ProcessingError => Status::internal(msg),
            ResyncClientError::MergeCommitError(merge_commit_error) => {
                error!(%merge_commit_error, "failed merging commit");
                Status::internal(msg)
            }
        }
    }
}
