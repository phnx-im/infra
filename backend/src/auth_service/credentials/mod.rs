// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::crypto::errors::KeyGenerationError;
use thiserror::Error;

use crate::persistence::StorageError;

pub(in crate::auth_service) mod intermediate_signing_key;
pub(in crate::auth_service) mod signing_key;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "credential_type", rename_all = "lowercase")]
enum CredentialType {
    As,
    Intermediate,
}

#[derive(Debug, Error)]
pub enum CredentialGenerationError {
    #[error("Can't sign new credential")]
    SigningError,
    #[error("No active credential")]
    NoActiveCredential,
    #[error(transparent)]
    CredentialGenerationFailed(#[from] KeyGenerationError),
    #[error(transparent)]
    StorageFailed(#[from] StorageError),
}

impl From<sqlx::Error> for CredentialGenerationError {
    fn from(e: sqlx::Error) -> Self {
        Self::StorageFailed(e.into())
    }
}
