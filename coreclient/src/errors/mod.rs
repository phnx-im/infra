// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::groups::GroupOperationError;

use phnxapiclient::ds_api::DsRequestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CorelibError {
    #[error(transparent)]
    Group(#[from] GroupOperationError),
    #[error(transparent)]
    DsError(#[from] DsRequestError),
}
