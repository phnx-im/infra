// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::errors::StorageError, memory_provider::MlsAssistMemoryStorage,
    openmls::group::MergeCommitError,
};
use thiserror::Error;

use crate::codec::PhnxCodec;

pub mod auth_service;
pub mod qs;

pub type CborMlsAssistStorage = MlsAssistMemoryStorage<PhnxCodec>;

/// Error updating queue config.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum UpdateQueueConfigError {
    /// Couldn't find sender.
    #[error("Couldn't find sender.")]
    UnknownSender,
}

/// Potential errors when performing a group operation.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum GroupOperationError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Missing queue config in client key package.
    #[error("Missing queue config in client key package.")]
    MissingQueueConfig,
    /// Failed to retrieve QS verifying key.
    #[error("Failed to retrieve QS verifying key.")]
    FailedToObtainVerifyingKey,
    /// Invalid KeyPackageBatch.
    #[error("Invalid KeyPackageBatch.")]
    InvalidKeyPackageBatch,
    /// User added twice.
    #[error("User added twice.")]
    DuplicatedUserAddition,
    /// Incomplete Welcome message.
    #[error("Incomplete Welcome message.")]
    IncompleteWelcome,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when updating a client.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum ClientUpdateError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Unknown sender.
    #[error("Unknown sender.")]
    UnknownSender,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when processing a message.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum DsProcessingError {
    /// Failed to distribute message to other members
    #[error("Failed to distribute message to other members")]
    DistributionError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Invalid signature.
    #[error("Invalid signature.")]
    InvalidSignature,
    /// Group not found.
    #[error("Group not found.")]
    GroupNotFound,
    /// Could not decrypt group state.
    #[error("Could not decrypt group state.")]
    CouldNotDecrypt,
    /// Could not encrypt group state.
    #[error("Could not decrypt group state.")]
    CouldNotEncrypt,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Unknown sender.
    #[error("Unknown sender.")]
    UnknownSender,
    /// Invalid sender type.
    #[error("Invalid sender type")]
    InvalidSenderType,
    /// Error storing encrypted group state.
    #[error("Error storing encrypted group state.")]
    StorageError,
    /// Error creating group.
    #[error("Failed to create group: Group ID not reserved")]
    UnreservedGroupId,
    /// Error updating client.
    #[error(transparent)]
    ClientUpdateError(#[from] ClientUpdateError),
    /// Could not find welcome info for this sender and/or this epoch.
    #[error("Could not find welcome info for this sender and/or this epoch.")]
    NoWelcomeInfoFound,
    /// Error joining group.
    #[error(transparent)]
    JoinGroupError(#[from] JoinGroupError),
    /// Error joining connection group.
    #[error(transparent)]
    JoinConnectionGroupError(#[from] JoinConnectionGroupError),
    /// Error adding clients.
    #[error(transparent)]
    ClientAddtionError(#[from] ClientAdditionError),
    /// Error removing clients.
    #[error(transparent)]
    ClientRemovalError(#[from] ClientRemovalError),
    /// Error resyncing client.
    #[error(transparent)]
    ClientResyncError(#[from] ResyncClientError),
    /// Error self removing client.
    #[error(transparent)]
    ClientSelfRemovalError(#[from] ClientSelfRemovalError),
    /// Error deleting group.
    #[error(transparent)]
    GroupDeletionError(#[from] GroupDeletionError),
    /// Error performing group operation.
    #[error(transparent)]
    GroupOperationError(#[from] GroupOperationError),
}

/// Potential errors when joining a group.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum JoinGroupError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Unknown sender.
    #[error("Unknown sender.")]
    UnknownSender,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when joining a connection group.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum JoinConnectionGroupError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Unknown sender.
    #[error("Unknown sender.")]
    UnknownSender,
    /// Not a connection group.
    #[error("Not a connection group.")]
    NotAConnectionGroup,
    /// User auth key collision.
    #[error("User auth key collision.")]
    UserAuthKeyCollision,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when adding a user.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum ClientAdditionError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Missing queue config in client key package.
    #[error("Missing queue config in client key package.")]
    MissingQueueConfig,
    /// Incomplete Welcome message.
    #[error("Incomplete Welcome message.")]
    IncompleteWelcome,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when removing clients.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum ClientRemovalError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when deleting a group.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum GroupDeletionError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when processing a self remove proposal.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum ClientSelfRemovalError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when sending a message.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum MessageSendingError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
}

/// Potential errors when resyncing a client.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum ResyncClientError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    #[error("Error merging commit: {0}")]
    MergeCommitError(#[from] MergeCommitError<StorageError<CborMlsAssistStorage>>),
}

/// Potential errors when validating a commit or proposal.
#[derive(Debug, Error)]
#[repr(u8)]
pub enum ValidationError {
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
}
