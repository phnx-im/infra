// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxtypes::codec::Error),
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
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// Mismatching sequence numbers.
    #[error("Mismatching sequence numbers.")]
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

impl From<phnxtypes::codec::Error> for QueueError {
    fn from(e: phnxtypes::codec::Error) -> Self {
        Self::Storage(e.into())
    }
}
