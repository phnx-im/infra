use std::fmt;

use argon2::Argon2;
use chrono::Duration;
use displaydoc::Display;
use thiserror::Error;

const MIN_USER_HANDLE_LENGTH: usize = 3;
const MAX_USER_HANDLE_LENGTH: usize = 46;
const USER_HANDLE_CHARSET: &[u8] = b"_0123456789abcdefghijklmnopqrstuvwxyz";

pub const USER_HANDLE_VALIDITY_PERIOD: Duration = Duration::days(30);

/// Validated plaintext user handle
#[derive(Clone)]
pub struct UserHandle {
    plaintext: String,
}

impl fmt::Debug for UserHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserHandle")
            .field("plaintext", &"<redacted>")
            .finish()
    }
}

impl UserHandle {
    pub fn new(plaintext: String) -> Result<Self, UserHandleValidationError> {
        Self::validate(&plaintext)?;
        Ok(Self { plaintext })
    }

    fn validate(plaintext: &str) -> Result<(), UserHandleValidationError> {
        if plaintext.len() < MIN_USER_HANDLE_LENGTH {
            return Err(UserHandleValidationError::TooShort);
        }
        if plaintext.len() > MAX_USER_HANDLE_LENGTH {
            return Err(UserHandleValidationError::TooLong);
        }
        for c in plaintext.bytes() {
            if !USER_HANDLE_CHARSET.contains(&c) {
                return Err(UserHandleValidationError::InvalidCharacter);
            }
        }
        Ok(())
    }

    pub fn into_plaintext(self) -> String {
        self.plaintext
    }

    pub fn hash(&self) -> Result<UserHandleHash, UserHandleHashError> {
        let argon2 = Argon2::default();
        let const_salt = b"user handle salt"; // TODO(security): this is not what we want
        let mut hash = [0u8; 32];
        argon2.hash_password_into(self.plaintext.as_bytes(), const_salt, &mut hash)?;
        Ok(UserHandleHash { hash })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserHandleHash {
    hash: [u8; 32],
}

impl UserHandleHash {
    pub fn new(hash: [u8; 32]) -> Self {
        Self { hash }
    }

    pub fn into_bytes(self) -> [u8; 32] {
        self.hash
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }
}

#[derive(Debug, Error, Display)]
pub enum UserHandleValidationError {
    /// User handle is too short
    TooShort,
    /// User handle is too long
    TooLong,
    /// User handle contains invalid characters
    InvalidCharacter,
}

#[derive(Debug, thiserror::Error)]
pub enum UserHandleHashError {
    #[error(transparent)]
    Argon2(#[from] argon2::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_user_handle_string() -> String {
        "test_user_123".to_string()
    }

    #[test]
    fn test_user_handle_new_valid() {
        let handle_str = valid_user_handle_string();
        let handle = UserHandle::new(handle_str.clone());
        assert!(handle.is_ok());
        assert_eq!(handle.unwrap().into_plaintext(), handle_str);
    }

    #[test]
    fn test_user_handle_new_too_short() {
        let handle_str = "ab".to_string(); // Length 2, MIN_USER_HANDLE_LENGTH is 3
        let handle = UserHandle::new(handle_str);
        assert!(handle.is_err());
        assert!(matches!(
            handle.unwrap_err(),
            UserHandleValidationError::TooShort
        ));
    }

    #[test]
    fn test_user_handle_new_too_long() {
        let handle_str = "a".repeat(MAX_USER_HANDLE_LENGTH + 1); // Length 47, MAX_USER_HANDLE_LENGTH is 46
        let handle = UserHandle::new(handle_str);
        assert!(handle.is_err());
        assert!(matches!(
            handle.unwrap_err(),
            UserHandleValidationError::TooLong
        ));
    }

    #[test]
    fn test_user_handle_new_invalid_character() {
        let handle_str = "user_handle!".to_string(); // '!' is not in USER_HANDLE_CHARSET
        let handle = UserHandle::new(handle_str);
        assert!(handle.is_err());
        assert!(matches!(
            handle.unwrap_err(),
            UserHandleValidationError::InvalidCharacter
        ));

        let handle_str_uppercase = "UserHandle".to_string(); // 'U' is not in USER_HANDLE_CHARSET
        let handle_uppercase = UserHandle::new(handle_str_uppercase);
        assert!(handle_uppercase.is_err());
        assert!(matches!(
            handle_uppercase.unwrap_err(),
            UserHandleValidationError::InvalidCharacter
        ));
    }

    #[test]
    fn test_user_handle_new_unicode_character() {
        let handle_str = "user_hændle".to_string(); // 'æ' is a Unicode character, not in USER_HANDLE_CHARSET
        let handle = UserHandle::new(handle_str);
        assert!(handle.is_err());
        assert!(matches!(
            handle.unwrap_err(),
            UserHandleValidationError::InvalidCharacter
        ));

        let handle_str_emoji = "user😊handle".to_string(); // Emoji is a Unicode character
        let handle_emoji = UserHandle::new(handle_str_emoji);
        assert!(handle_emoji.is_err());
        assert!(matches!(
            handle_emoji.unwrap_err(),
            UserHandleValidationError::InvalidCharacter
        ));
    }

    #[test]
    fn test_user_handle_debug_redacted() {
        let handle = UserHandle::new(valid_user_handle_string()).unwrap();
        let debug_output = format!("{:?}", handle);
        assert!(debug_output.contains("<redacted>"));
        assert!(!debug_output.contains("test_user_123")); // Ensure original plaintext is not visible
    }

    #[test]
    fn test_user_handle_into_plaintext() {
        let handle_str = valid_user_handle_string();
        let handle = UserHandle::new(handle_str.clone()).unwrap();
        let extracted_plaintext = handle.into_plaintext();
        assert_eq!(extracted_plaintext, handle_str);
    }

    #[test]
    fn test_user_handle_hash_produces_hash() {
        let handle = UserHandle::new(valid_user_handle_string()).unwrap();
        let handle_hash = handle.hash().unwrap();
        assert_eq!(
            hex::encode(handle_hash.hash),
            "67eedaa506238ce0774d7ee8bbda5cf5bef329607dbbad4c2cccd96ae8024a76"
        );
    }

    #[test]
    fn test_user_handle_hash_consistency() {
        // Hashing the same input with an empty salt should produce the same hash
        let handle_str = valid_user_handle_string();
        let handle1 = UserHandle::new(handle_str.clone()).unwrap();
        let handle2 = UserHandle::new(handle_str).unwrap();

        let hash1 = handle1.hash().unwrap();
        let hash2 = handle2.hash().unwrap();

        assert_eq!(hash1.hash, hash2.hash);
    }
}
