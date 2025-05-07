// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Display;

use phnxtypes::identifiers::{TlsStr, TlsString};
use thiserror::Error;

use super::*;

/// A display name is a human-readable name that can be used to identify a user.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DisplayName {
    display_name: String,
}

const MAX_DISPLAY_NAME_LENGTH: usize = 50;

impl TryFrom<String> for DisplayName {
    type Error = DisplayNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Check if the display name is empty.
        if value.is_empty() {
            return Err(DisplayNameError::DisplayNameEmpty);
        }
        if value.len() > MAX_DISPLAY_NAME_LENGTH {
            return Err(DisplayNameError::DisplayNameTooLong);
        }
        // Trim whitespace at beginning and end.
        let value = value.trim();
        // Pad with spaces to the maximum length.
        let mut padded_display_name = String::new();
        padded_display_name.push_str(value);
        padded_display_name.push_str(&" ".repeat(MAX_DISPLAY_NAME_LENGTH - value.len()));
        Ok(Self {
            display_name: padded_display_name,
        })
    }
}

#[derive(Debug, Error)]
pub enum DisplayNameError {
    #[error("Display name is too long")]
    DisplayNameTooLong,
    #[error("Display name is empty")]
    DisplayNameEmpty,
}

impl DisplayName {
    fn trimmed(&self) -> &str {
        // Return the trimmed version of the display name.
        self.display_name.trim()
    }
}

impl sqlx::Type<Sqlite> for DisplayName {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for DisplayName {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(&self.display_name, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for DisplayName {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let display_name: String = Decode::<Sqlite>::decode(value)?;
        Ok(Self { display_name })
    }
}

impl Display for DisplayName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.trimmed())
    }
}

impl AsRef<str> for DisplayName {
    fn as_ref(&self) -> &str {
        // Use the trimmed version of the display name.
        self.trimmed()
    }
}

impl tls_codec::Size for DisplayName {
    fn tls_serialized_len(&self) -> usize {
        TlsStr(&self.display_name).tls_serialized_len()
    }
}

impl tls_codec::Serialize for DisplayName {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        TlsStr(&self.display_name).tls_serialize(writer)
    }
}

impl tls_codec::DeserializeBytes for DisplayName {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (TlsString(display_name), bytes) = TlsString::tls_deserialize_bytes(bytes)?;
        let display_name = DisplayName { display_name };
        Ok((display_name, bytes))
    }
}
