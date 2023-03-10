// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub struct Username {
    text: String,
}

impl Username {
    pub fn from_text(text: &str) -> Result<Self, UsernameValidationError> {
        // TODO: validate username
        Ok(Username {
            text: text.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

pub enum UsernameValidationError {
    Invalid,
}
