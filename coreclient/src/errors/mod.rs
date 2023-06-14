// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[macro_use]
pub(crate) mod error_macros;

use crate::{
    conversations::ConversationStoreError,
    groups::{GroupOperationError, GroupStoreError},
};

use phnxapiclient::ds_api::DsRequestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CorelibError {
    #[error("The backend is not initialized.")]
    BackendNotInitialized,
    #[error("A network error occurred")]
    NetworkError,
    #[error("KeyPackage received from backend is invalid")]
    InvalidKeyPackage,
    #[error("User not initialized")]
    UserNotInitialized,
    #[error(transparent)]
    Group(#[from] GroupOperationError),
    #[error(transparent)]
    GroupStore(#[from] GroupStoreError),
    #[error(transparent)]
    ConversationStore(#[from] ConversationStoreError),
    #[error(transparent)]
    DsError(#[from] DsRequestError),
}

#[derive(Error, Debug)]
pub enum GrpcError {
    #[error("Missing parameter in the request")]
    MissingParameter,
}
