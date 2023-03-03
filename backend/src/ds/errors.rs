use mls_assist::GroupEpoch;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum GroupCreationError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid group id
    #[error("Invalid group id")]
    InvalidGroupId, // Wrong group ID [too explicit]
    // TODO: Probably a bit too vague. Not making this transparent for now, as we'd have to derive serde for a number of OpenMLS errors.
    /// Invalid group creation parameters.
    #[error("Invalid group creation parameters.")]
    InvalidParameters,
    /// Error storing group state.
    #[error("Error storing group state.")]
    StorageError,
    /// Could not encrypt group state.
    #[error("Could not decrypt group state.")]
    CouldNotEncrypt,
}

/// Error struct for the verification of [`UnverifiedGroupOperationParams`]
/// instances.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum GroupOperationVerificationError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Wrong group ID
    #[error("Wrong group ID")]
    WrongGroupId, // Wrong group ID [too explicit]
    /// Group expired
    #[error("Group expired")]
    GroupExpired, // Group expired [too explicit]
    /// Failed to decrypt roster with given key
    #[error("Failed to decrypt roster with given key")]
    WrongRosterKey,
    /// No client with the given index could be found in roster
    #[error("No client with the given index could be found in roster")]
    UnknownSender,
    /// The sender has not yet committed to the group and thus can't send application messages
    #[error(
        "The sender has not yet committed to the group and thus can't send application messages"
    )]
    UninitializedSender,
    /// Failed to authenticate sender
    #[error("Failed to authenticate sender")]
    MemberAuthenticationFailed,
}

/// Error struct for processing of DS group operations.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum GroupOperationProcessingError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Request verification failed
    #[error("Request verification failed")]
    RequestVerificationFailure(GroupOperationVerificationError),
    /// Failed to send non-commit message
    #[error("Failed to send non-commit message")]
    NonCommitSendingFailure(DistributeNonCommitError),
    /// Failed to send commit message
    #[error("Failed to send commit message")]
    CommitSendingFailure(ProcessCommitError),
    /// Failed to update queue config
    #[error("Failed to update queue config")]
    QueueConfigUpdateFailure(UpdateQueueConfigError),
}

impl From<GroupOperationVerificationError> for GroupOperationProcessingError {
    fn from(e: GroupOperationVerificationError) -> Self {
        if matches!(e, GroupOperationVerificationError::LibraryError) {
            Self::LibraryError
        } else {
            Self::RequestVerificationFailure(e)
        }
    }
}

impl From<DistributeNonCommitError> for GroupOperationProcessingError {
    fn from(e: DistributeNonCommitError) -> Self {
        if matches!(e, DistributeNonCommitError::LibraryError) {
            Self::LibraryError
        } else {
            Self::NonCommitSendingFailure(e)
        }
    }
}

/// Group creation error.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CreateGroupError {
    /// Library Error
    #[error("LibraryError")]
    LibraryError,
    /// The signature is invalid.
    #[error("The signature is invalid.")]
    InvalidSignature,
    /// Insufficient randomness
    #[error("Insufficient randomness.")]
    InsufficientRandomness,
    /// Storage provider failed to store group state.
    #[error("Storage provider failed to store group state.")]
    StorageError,
}

/// Group encryption error.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EncryptGroupError {
    /// Encryption error
    #[error("Encryption error")]
    EncryptionError,
    /// Insufficient randomness to encrypt
    #[error("Insufficient randomness to encrypt")]
    InsufficientRandomness,
}

/// Error updating queue config.
#[derive(Error, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum UpdateQueueConfigError {
    /// LibraryError
    #[error("LibraryError")]
    LibraryError,
    /// Insufficient randomness
    #[error("Insufficient randomness.")]
    InsufficientRandomness,
    /// Error storing updated group state
    #[error("Error storing updated group state.")]
    StorageError,
    /// Couldn't find sender in the roster
    #[error("Couldn't find sender in the roster")]
    UnknownSender,
}

/// Error sending non-commit messages.
#[derive(Error, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ProcessCommitError {
    /// There is a mismatch between the message epoch and the group epoch.
    #[error("There is a mismatch between the message epoch and the group epoch.")]
    EpochMismatch,
    /// The sender does not have sufficient privileges for the desired roster operation.
    #[error("The sender does not have sufficient privileges for the desired roster operation.")]
    InsufficientPrivileges,
    /// Could not apply the given modifications to the roster.
    #[error("Could not apply the given modifications to the roster.")]
    InvalidRosterDelta,
    /// Error distributing the message.
    #[error("Error distributing the message.")]
    DistributionError,
    /// LibraryError
    #[error("LibraryError")]
    LibraryError,
    /// Insufficient randomness
    #[error("Insufficient randomness.")]
    InsufficientRandomness,
    /// Error storing updated group state
    #[error("Error storing updated group state.")]
    StorageError,
}

/// Error distributing messages.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub(super) enum MessageDistributionError {
    /// Error delivering the message to the QS
    #[error("DeliveryError")]
    DeliveryError,
}

/// Potential errors when distributing a non-commit message.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DistributeNonCommitError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Failed to distribute message to other members
    #[error("Failed to distribute message to other members")]
    DistributionError,
    /// Message epoch doesn't match group epoch
    #[error("Message epoch doesn't match group epoch")]
    WrongEpoch(GroupEpoch, GroupEpoch),
}

/// Potential errors when adding a user.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum UserAdditionError {
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

/// Potential errors when processing a message.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DsProcessingError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
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
    /// Error adding user.
    #[error(transparent)]
    AddUserError(#[from] UserAdditionError),
}
