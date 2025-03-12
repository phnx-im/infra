// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides structs and functions to interact with users in the
//! various groups an InfraClient is a member of.

use std::fmt::Display;

use phnxtypes::identifiers::QualifiedUserName;
use rusqlite::{ToSql, types::FromSql};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

pub(crate) mod persistence;

/// A user profile contains information about a user, such as their display name
/// and profile picture.
#[derive(
    Debug, Clone, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub struct UserProfile {
    user_name: QualifiedUserName,
    display_name_option: Option<DisplayName>,
    profile_picture_option: Option<Asset>,
}

impl UserProfile {
    pub fn new(
        user_name: QualifiedUserName,
        display_name_option: Option<DisplayName>,
        profile_picture_option: Option<Asset>,
    ) -> Self {
        Self {
            user_name,
            display_name_option,
            profile_picture_option,
        }
    }

    pub fn user_name(&self) -> &QualifiedUserName {
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

/// A display name is a human-readable name that can be used to identify a user.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
