// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::utils::persistence::PersistenceError;

use super::*;

use openmls_memory_keystore::MemoryKeyStoreError;
use phnxbackend::crypto::DecryptionError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum GroupOperationError {
    #[error(transparent)]
    MergeCommitError(#[from] MergeCommitError<MemoryKeyStoreError>),
    #[error(transparent)]
    WelcomeError(#[from] WelcomeError<MemoryKeyStoreError>),
    #[error(transparent)]
    MlsGroupStateError(#[from] MlsGroupStateError),
    #[error(transparent)]
    CreateMessageError(#[from] CreateMessageError),
    #[error(transparent)]
    ProcessMessageError(#[from] ProcessMessageError),
    #[error(transparent)]
    AddMembersError(#[from] AddMembersError<MemoryKeyStoreError>),
    #[error(transparent)]
    MergePendingCommitError(#[from] MergePendingCommitError<MemoryKeyStoreError>),
    #[error("Missing key package in key store")]
    MissingKeyPackage,
    #[error(transparent)]
    JoinerInfoDecryptionError(#[from] DecryptionError),
    #[error(transparent)]
    TlsCodecError(#[from] tls_codec::Error),
    #[error(transparent)]
    GroupStoreError(#[from] PersistenceError),
    #[error("Invalid group state: Can't find own client information")]
    InvalidGroupState,
}
