// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("Error deserializing column: {0}")]
    Serde(#[from] phnxtypes::codec::Error),
}
