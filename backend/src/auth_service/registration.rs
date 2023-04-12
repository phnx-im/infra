// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::*;

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("Username is not valid")]
    UsernameInvalid,
    #[error("Username is already taken")]
    UsernameTaken,
    #[error("An internal server error occurred")]
    ServerError,
}
