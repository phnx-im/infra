use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error updating queue config.
#[derive(Error, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum UpdateQueueConfigError {
    /// Couldn't find sender.
    #[error("Couldn't find sender.")]
    UnknownSender,
}

/// Error distributing messages.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub(super) enum MessageDistributionError {
    /// Error delivering the message to the QS
    #[error("DeliveryError")]
    DeliveryError,
}

/// Potential errors when removing users.
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum UserRemovalError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid assisted message.
    #[error("Invalid assisted message.")]
    InvalidMessage,
    /// Error processing message.
    #[error("Error processing message.")]
    ProcessingError,
    /// Commit didn't cover all clients of a user.
    #[error("Commit didn't cover all clients of a user.")]
    IncompleteRemoval,
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
}

/// Potential errors when updating a client.
#[derive(Debug, Error, Serialize, Deserialize)]
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
    /// Error adding users.
    #[error(transparent)]
    AddUsersError(#[from] UserAdditionError),
    /// Error removing users.
    #[error(transparent)]
    RemoveUsersError(#[from] UserRemovalError),
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
}

/// Potential errors when joining a group.
#[derive(Debug, Error, Serialize, Deserialize)]
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
}

/// Potential errors when joining a connection group.
#[derive(Debug, Error, Serialize, Deserialize)]
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
}
