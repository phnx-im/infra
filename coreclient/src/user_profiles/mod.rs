// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides structs and functions to interact with users in the
//! various groups an InfraClient is a member of.

use std::fmt::Display;

use phnxtypes::identifiers::{SafeTryInto, UserName};
use rusqlite::{params, types::FromSql, Connection, OptionalExtension, ToSql};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    conversations::messages::TimestampedMessage,
    utils::persistence::{Storable, Triggerable},
    ConversationId, EventMessage, Message, SystemMessage,
};

/// A user profile contains information about a user, such as their display name
/// and profile picture.
#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    user_name: UserName,
    display_name_option: Option<DisplayName>,
    profile_picture_option: Option<Asset>,
}

impl UserProfile {
    pub fn new(
        user_name: UserName,
        display_name_option: Option<DisplayName>,
        profile_picture_option: Option<Asset>,
    ) -> Self {
        Self {
            user_name,
            display_name_option,
            profile_picture_option,
        }
    }

    pub(crate) fn load(
        connection: &Connection,
        user_name: &UserName,
    ) -> Result<Option<Self>, rusqlite::Error> {
        let mut statement = connection.prepare(
            "SELECT user_name, display_name, profile_picture FROM users WHERE user_name = ?",
        )?;
        let user = statement
            .query_row(params![user_name.to_string()], Self::from_row)
            .optional()?;

        if let Some(user_profile) = &user {
            if user_name != user_profile.user_name() {
                // This should never happen, but if it does, we want to know about it.
                log::error!(
                    "User name mismatch: Expected {}, got {}",
                    user_name,
                    user_profile.user_name()
                );
            }
        }
        Ok(user)
    }

    /// Store the user's profile in the database. This will overwrite any existing profile.
    ///
    /// This function is intended to be used for storing the user's own profile. To store a user
    /// profile as a member of a group, use [`register_as_conversation_participant`] instead.
    pub(crate) fn store_own_user_profile(
        connection: &Connection,
        user_name: UserName,
        display_name_option: Option<DisplayName>,
        profile_picture_option: Option<Asset>,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR REPLACE INTO users (user_name, display_name, profile_picture) VALUES (?, ?, ?)",
            params![
                user_name.to_string(),
                display_name_option,
                profile_picture_option
            ],
        )?;
        Ok(())
    }

    /// Update the user's display name and profile picture in the database. To store a new profile,
    /// use [`register_as_conversation_participant`] instead.
    pub(crate) fn update(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "UPDATE users SET display_name = ?2, profile_picture = ?3 WHERE user_name = ?1",
            params![
                self.user_name.to_string(),
                self.display_name_option,
                self.profile_picture_option
            ],
        )?;
        Ok(())
    }

    /// Stores this new [`UserProfile`] if one doesn't already exist.
    pub(crate) fn store(&self, connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute(
            "INSERT OR IGNORE INTO users (user_name, display_name, profile_picture) VALUES (?, ?, ?)",
            params![
                self.user_name.to_string(),
                self.display_name_option,
                self.profile_picture_option
            ],
        )?;
        Ok(())
    }

    pub fn user_name(&self) -> &UserName {
        &self.user_name
    }

    pub fn display_name(&self) -> Option<&DisplayName> {
        self.display_name_option.as_ref()
    }

    pub fn profile_picture(&self) -> Option<&Asset> {
        self.profile_picture_option.as_ref()
    }

    pub fn set_display_name(&mut self, display_name: Option<DisplayName>) {
        self.display_name_option = display_name;
    }

    pub fn set_profile_picture(&mut self, profile_picture: Option<Asset>) {
        self.profile_picture_option = profile_picture;
    }
}

impl Storable for UserProfile {
    const CREATE_TABLE_STATEMENT: &'static str = "CREATE TABLE IF NOT EXISTS users (
                user_name TEXT PRIMARY KEY,
                display_name TEXT,
                profile_picture BLOB
            )";

    fn from_row(row: &rusqlite::Row) -> anyhow::Result<Self, rusqlite::Error> {
        let user_name = row.get(0)?;
        let display_name_option = row.get(1)?;
        let profile_picture_option = row.get(2)?;
        Ok(UserProfile {
            user_name,
            display_name_option,
            profile_picture_option,
        })
    }
}

/// A display name is a human-readable name that can be used to identify a user.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DisplayName {
    display_name: String,
}

impl FromSql for DisplayName {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let display_name = String::column_result(value)?;
        Ok(Self { display_name })
    }
}

impl ToSql for DisplayName {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.display_name.to_sql()
    }
}

#[derive(Debug, Error)]
pub enum DisplayNameError {
    #[error("Invalid display name")]
    InvalidDisplayName,
}

// We might want to add more constraints here, e.g. on the length of the display
// name.
impl TryFrom<String> for DisplayName {
    type Error = DisplayNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self {
            display_name: value,
        })
    }
}

impl Display for DisplayName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

impl AsRef<str> for DisplayName {
    fn as_ref(&self) -> &str {
        &self.display_name
    }
}

impl tls_codec::Size for DisplayName {
    fn tls_serialized_len(&self) -> usize {
        self.display_name.as_bytes().tls_serialized_len()
    }
}

impl tls_codec::Serialize for DisplayName {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        self.display_name.as_bytes().tls_serialize(writer)
    }
}

impl tls_codec::DeserializeBytes for DisplayName {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (display_name_bytes, bytes): (Vec<u8>, &[u8]) =
            tls_codec::DeserializeBytes::tls_deserialize_bytes(bytes)?;
        let display_name = String::from_utf8(display_name_bytes.to_vec()).map_err(|_| {
            tls_codec::Error::DecodingError("Couldn't convert bytes to UTF-8 string".to_string())
        })?;
        Ok((DisplayName { display_name }, bytes))
    }
}

#[derive(
    Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
#[repr(u8)]
pub enum Asset {
    Value(Vec<u8>),
    // TODO: Assets by Reference
}

impl ToSql for Asset {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Asset::Value(value) => value.to_sql(),
        }
    }
}

impl FromSql for Asset {
    fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
        let value = Vec::<u8>::column_result(value)?;
        Ok(Asset::Value(value))
    }
}

impl Asset {
    pub fn value(&self) -> Option<&[u8]> {
        match self {
            Asset::Value(value) => Some(value),
        }
    }
}
