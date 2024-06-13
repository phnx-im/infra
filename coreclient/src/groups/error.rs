// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::utils::persistence::PersistenceError;

use mls_assist::messages::AssistedMessageError;
use openmls::group::{
    AddMembersError, CreateMessageError, MergeCommitError, MergePendingCommitError,
    MlsGroupStateError, ProcessMessageError, WelcomeError,
};
use phnxtypes::crypto::DecryptionError;
use thiserror::Error;

use super::openmls_provider::storage_provider::SqliteStorageProviderError;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum GroupOperationError {
    #[error(transparent)]
    MergeCommitError(#[from] MergeCommitError<SqliteStorageProviderError>),
    #[error(transparent)]
    WelcomeError(#[from] WelcomeError<SqliteStorageProviderError>),
    #[error(transparent)]
    MlsGroupStateError(#[from] MlsGroupStateError<SqliteStorageProviderError>),
    #[error(transparent)]
    CreateMessageError(#[from] CreateMessageError<SqliteStorageProviderError>),
    #[error(transparent)]
    ProcessMessageError(#[from] ProcessMessageError<SqliteStorageProviderError>),
    #[error(transparent)]
    AddMembersError(#[from] AddMembersError<SqliteStorageProviderError>),
    #[error(transparent)]
    MergePendingCommitError(#[from] MergePendingCommitError<SqliteStorageProviderError>),
    #[error("Missing key package in key store")]
    MissingKeyPackage,
    #[error(transparent)]
    JoinerInfoDecryptionError(#[from] DecryptionError),
    #[error(transparent)]
    TlsCodecError(#[from] tls_codec::Error),
    #[error(transparent)]
    GroupStoreError(#[from] PersistenceError),
    #[error(transparent)]
    AssistedMessageError(#[from] AssistedMessageError),
}
