// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::messages::AssistedMessageError;
use openmls::group::{
    AddMembersError, CreateMessageError, MergeCommitError, MergePendingCommitError,
    MlsGroupStateError, ProcessMessageError, WelcomeError,
};
use phnxtypes::crypto::DecryptionError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum GroupOperationError {
    #[error(transparent)]
    MergeCommitError(#[from] MergeCommitError<rusqlite::Error>),
    #[error(transparent)]
    WelcomeError(#[from] WelcomeError<rusqlite::Error>),
    #[error(transparent)]
    MlsGroupStateError(#[from] MlsGroupStateError<rusqlite::Error>),
    #[error(transparent)]
    CreateMessageError(#[from] CreateMessageError<rusqlite::Error>),
    #[error(transparent)]
    ProcessMessageError(#[from] ProcessMessageError<rusqlite::Error>),
    #[error(transparent)]
    AddMembersError(#[from] AddMembersError<rusqlite::Error>),
    #[error(transparent)]
    MergePendingCommitError(#[from] MergePendingCommitError<rusqlite::Error>),
    #[error("Missing key package in key store")]
    MissingKeyPackage,
    #[error(transparent)]
    JoinerInfoDecryptionError(#[from] DecryptionError),
    #[error(transparent)]
    TlsCodecError(#[from] tls_codec::Error),
    #[error(transparent)]
    AssistedMessageError(#[from] AssistedMessageError),
}
