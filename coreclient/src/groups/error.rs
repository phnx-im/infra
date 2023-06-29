// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

use openmls_memory_keystore::MemoryKeyStoreError;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum GroupOperationError {
    #[error("LibraryError")]
    LibraryError,
    #[error("Could not invite user to group")]
    InvitationError,
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
    #[error("No pending group diff")]
    NoPendingGroupDiff,
    #[error("User already in group")]
    DuplicateUserAddition,
    #[error("Client already in group")]
    DuplicateClientAddition,
}

implement_error! {
    pub enum GroupStoreError {
        Simple {
            InsertionError = "Could not insert new group into store",
            DuplicateGroup = "This group already exists",
            UnknownGroup = "This group does not exist",
        }
        Complex {}
    }
}
