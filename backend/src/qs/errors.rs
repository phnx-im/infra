// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{network_provider_trait::NetworkProvider, storage_provider_trait::QsStorageProvider};
use phnxtypes::crypto::errors::DecryptionError;
use thiserror::Error;

// === DS API errors ===

/// Error fetching a message from the QS.
#[derive(Error, Debug)]
pub enum QsEnqueueError<S: QsStorageProvider, N: NetworkProvider> {
    /// Couldn't find the requested queue.
    #[error("Couldn't find the requested queue")]
    QueueNotFound,
    /// Unseal error
    #[error(transparent)]
    UnsealError(#[from] DecryptionError),
    /// An error ocurred enqueueing in a fan out queue
    #[error(transparent)]
    EnqueueError(#[from] EnqueueError<S>),
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
pub enum EnqueueError<S: QsStorageProvider> {
    /// Unrecoverable implementation error
    #[error("Library Error")]
    LibraryError, // E.g. an error while encoding a message before enqueing it.
    /// Error in the underlying storage provider
    #[error("Error in the underlying storage provider: {0}")]
    StorageProviderEnqueueError(S::EnqueueError),
    /// Error in the underlying storage provider
    #[error("Error in the underlying storage provider: {0}")]
    StorageProviderStoreClientError(S::StoreClientError),
    /// Error sending push notification.
    #[error("Error sending push notification.")]
    PushNotificationError,
}
