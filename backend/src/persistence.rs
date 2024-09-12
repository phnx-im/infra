// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    DatabaseError(#[from] DatabaseError),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxtypes::codec::Error),
}

impl From<sqlx::Error> for StorageError {
    fn from(e: sqlx::Error) -> Self {
        Self::DatabaseError(e.into())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for StorageError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::DatabaseError(e.into())
    }
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Dynamic(#[from] Box<dyn std::error::Error + Send + Sync>),
}
