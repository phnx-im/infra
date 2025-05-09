// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt::Display, str::FromStr};

use phnxtypes::identifiers::{TlsStr, TlsString};
use thiserror::Error;

use super::*;

/// A display name is a human-readable name that can be used to identify a user.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DisplayName {
    display_name: String,
}

// Note that this counts chars, not graphemes. While chars are at most 4 bytes,
// graphemes can be longer, so we need to adjust the logic if we ever want to
// count graphemes instead of chars.
const MAX_DISPLAY_NAME_CHARS: usize = 50;
const DISALLOWED_CHARACTERS: [char; 3] = ['\r', '\n', '\t'];

impl FromStr for DisplayName {
    type Err = DisplayNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Trim whitespace at beginning and end.
        let value = s.trim();
        // Check if the display name is empty.
        if value.is_empty() {
            return Err(DisplayNameError::DisplayNameEmpty);
        }
        // If there are fewer than 50 chars, it also has fewer than 200 bytes.
        if value.chars().count() > MAX_DISPLAY_NAME_CHARS {
            return Err(DisplayNameError::DisplayNameTooLong);
        }
        if value.chars().any(|c| DISALLOWED_CHARACTERS.contains(&c)) {
            return Err(DisplayNameError::InvalidCharacters);
        }
        Ok(Self {
            display_name: value.to_string(),
        })
    }
}

#[derive(Debug, Error)]
pub enum DisplayNameError {
    #[error("Display name is too long")]
    DisplayNameTooLong,
    #[error("Display name is empty")]
    DisplayNameEmpty,
    #[error("Display name contains invalid characters")]
    InvalidCharacters,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ascii_under_limit() {
        let name = "Alice";
        let dn = DisplayName::from_str(name).unwrap();
        assert_eq!(&dn.display_name[..5], name);
    }

    #[test]
    fn rejects_empty_input() {
        let result = DisplayName::from_str("   ");
        assert!(matches!(result, Err(DisplayNameError::DisplayNameEmpty)));
        assert_eq!(result.unwrap_err().to_string(), "Display name is empty");
    }

    #[test]
    fn accepts_exactly_50_ascii_chars() {
        let name = "a".repeat(MAX_DISPLAY_NAME_CHARS);
        let dn = DisplayName::from_str(&name).unwrap();
        assert_eq!(&dn.display_name[..MAX_DISPLAY_NAME_CHARS], name);
    }

    #[test]
    fn rejects_more_than_50_chars() {
        let name = "a".repeat(MAX_DISPLAY_NAME_CHARS + 1);
        let result = DisplayName::from_str(&name);
        assert!(matches!(result, Err(DisplayNameError::DisplayNameTooLong)));
        assert_eq!(result.unwrap_err().to_string(), "Display name is too long");
    }

    #[test]
    fn accepts_emoji_upto_200_bytes() {
        let name = "🦀".repeat(MAX_DISPLAY_NAME_CHARS); // 4 bytes per char
        let dn = DisplayName::from_str(&name).unwrap();
        assert_eq!(dn.display_name.chars().count(), MAX_DISPLAY_NAME_CHARS);
    }

    #[test]
    fn rejects_emoji_over_200_bytes() {
        let name = "🦀".repeat(MAX_DISPLAY_NAME_CHARS + 1); // 204 bytes
        let result = DisplayName::from_str(&name);
        assert!(matches!(result, Err(DisplayNameError::DisplayNameTooLong)));
        assert_eq!(result.unwrap_err().to_string(), "Display name is too long");
    }

    #[test]
    fn trims_whitespace_correctly() {
        let name = "  hello\t  ";
        let dn = DisplayName::from_str(name).unwrap();
        assert!(dn.display_name.starts_with("hello"));
    }

    #[test]
    fn accepts_right_to_left_script() {
        // Arabic: "سلام" (salaam = peace)
        let name = "سلام"; // 4 Arabic characters
        let dn = DisplayName::from_str(name).unwrap();

        // Check that the characters are preserved correctly
        assert!(dn.display_name.starts_with(name));
    }

    #[test]
    fn trims_whitespace_in_rtl_string() {
        // Arabic: "سلام" (salaam = peace)
        let input = "  سلام  "; // 4 Arabic characters

        // "مرحبا" = 5 Arabic characters = 10 bytes in UTF-8
        let expected_trimmed = "سلام";

        let dn = DisplayName::from_str(input).unwrap();

        // Check: trimmed correctly
        assert!(dn.display_name.starts_with(expected_trimmed));
    }

    #[test]
    fn accepts_grapheme_clusters_over_4_bytes() {
        // A single emoji flag grapheme cluster: 2 code points, 8 bytes
        let name = "🇺🇳"; // UN flag
        assert_eq!(name.chars().count(), 2);
        assert_eq!(name.len(), 8);

        // Repeat 25 times = 50 scalar values, 200 bytes
        let full = name.repeat(25);
        assert_eq!(full.chars().count(), MAX_DISPLAY_NAME_CHARS);

        let dn = DisplayName::from_str(&full).unwrap();
        assert!(dn.display_name.starts_with(&full));
    }

    #[test]
    fn rejects_display_name_with_disallowed_characters() {
        for disallowed_char in DISALLOWED_CHARACTERS {
            let name = format!("hello{}world", disallowed_char);
            let result = DisplayName::from_str(&name);
            assert!(
                matches!(result, Err(DisplayNameError::InvalidCharacters)),
                "Expected error for input: {:?}, got: {:?}",
                name,
                result
            );
        }
        // Test case with more than one and different disallowed characters
        let name = format!(
            "foo{}bar{}baz",
            DISALLOWED_CHARACTERS[0], DISALLOWED_CHARACTERS[1]
        );
        let result = DisplayName::from_str(&name);
        assert!(
            matches!(result, Err(DisplayNameError::InvalidCharacters)),
            "Expected error for input: {:?}, got: {:?}",
            name,
            result
        );
    }
}
