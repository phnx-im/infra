// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::identifiers::UserName;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::utils::persistence::{DataType, Persistable, PersistableStruct, PersistenceError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayName {
    display_name: String,
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
    fn tls_deserialize(bytes: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (display_name_bytes, bytes): (Vec<u8>, &[u8]) =
            tls_codec::DeserializeBytes::tls_deserialize(bytes)?;
        let display_name = String::from_utf8(display_name_bytes.to_vec()).map_err(|_| {
            tls_codec::Error::DecodingError("Couldn't convert bytes to UTF-8 string".to_string())
        })?;
        Ok((DisplayName { display_name }, bytes))
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum Asset {
    Value(Vec<u8>),
    // TODO: Assets by Reference
}

impl Asset {
    pub fn value(&self) -> Option<&[u8]> {
        match self {
            Asset::Value(value) => Some(value),
        }
    }
}

#[derive(Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    display_name: DisplayName,
    profile_picture_option: Option<Asset>,
}

impl UserProfile {
    pub(crate) fn new(display_name: String, profile_picture_option: Option<Vec<u8>>) -> Self {
        let profile_picture_option = profile_picture_option.map(Asset::Value);
        Self {
            display_name: DisplayName { display_name },
            profile_picture_option,
        }
    }

    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }

    pub fn profile_picture_option(&self) -> Option<&Asset> {
        self.profile_picture_option.as_ref()
    }
}

impl From<UserName> for UserProfile {
    fn from(user_name: UserName) -> Self {
        Self::new(user_name.to_string(), None)
    }
}

impl Persistable for UserProfile {
    type Key = DataType;

    type SecondaryKey = DataType;

    const DATA_TYPE: DataType = DataType::UserProfile;

    fn key(&self) -> &Self::Key {
        &Self::DATA_TYPE
    }

    fn secondary_key(&self) -> &Self::SecondaryKey {
        &Self::DATA_TYPE
    }
}

type PersistableUserProfile<'a> = PersistableStruct<'a, UserProfile>;

impl PersistableUserProfile<'_> {
    pub(crate) fn into_payload(self) -> UserProfile {
        self.payload
    }
}

pub(crate) struct UserProfileStore<'a> {
    connection: &'a Connection,
}

impl<'a> From<&'a Connection> for UserProfileStore<'a> {
    fn from(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl UserProfileStore<'_> {
    pub(crate) fn store(&self, user_profile: UserProfile) -> Result<(), PersistenceError> {
        PersistableUserProfile::from_connection_and_payload(self.connection, user_profile).persist()
    }

    /// Loads the user profile of the user. If the user profile does not exist, returns `None`.
    pub(crate) fn get(&self) -> Result<Option<UserProfile>, PersistenceError> {
        let user_profile =
            PersistableUserProfile::load_one(self.connection, Some(&DataType::UserProfile), None)?;
        Ok(user_profile.map(|p| p.into_payload()))
    }
}
