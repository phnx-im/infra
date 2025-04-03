// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::errors::StorageError;

use super::network_provider::NetworkProvider;
use phnxtypes::crypto::errors::{DecryptionError, KeyGenerationError};
use thiserror::Error;

// === DS API errors ===

/// Error fetching a message from the QS.
#[derive(Error, Debug)]
pub enum QsEnqueueError<N: NetworkProvider> {
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// Unseal error
    #[error(transparent)]
    UnsealError(#[from] DecryptionError),
    /// An error ocurred enqueueing in a fan out queue
    #[error(transparent)]
    EnqueueError(#[from] EnqueueError),
    /// An error ocurred while sending a message to the network
    #[error("An error ocurred while sending a message to the network")]
    NetworkError(N::NetworkError),
    /// Storage provider error
    #[error("Storage provider error")]
    StorageError,
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError,
    /// Invalid response
    #[error("Invalid response")]
    InvalidResponse,
}

/// Error enqueuing a fanned-out message.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EnqueueError {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError, // E.g. an error while encoding a message before enqueing it.
    /// Error in the underlying storage provider
    #[error("Error in the underlying storage provider")]
    Storage,
    /// Error sending push notification.
    #[error("Error sending push notification.")]
    PushNotificationError,
}

// === Internal errors ===

#[derive(Debug, Error)]
pub(super) enum GenerateAndStoreError {
    #[error("Error generating signature keypair")]
    KeyGenerationError(#[from] KeyGenerationError),
    #[error("Error storing key")]
    StorageError(#[from] StorageError),
}
