// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::*;

#[derive(Debug, Error)]
pub enum InviteUserError {
    #[error("User not found")]
    UserNotFound,
    #[error("Wrong devices")]
    WrongDevices,
}
