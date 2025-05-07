// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides structs and functions to interact with users in the
//! various groups an InfraClient is a member of.

pub use display_name::{DisplayName, DisplayNameError};
use phnxtypes::{
    crypto::{
        ear::{EarDecryptable, EarEncryptable},
        indexed_aead::{
            ciphertexts::{IndexDecryptable, IndexEncryptable},
            keys::{UserProfileKey, UserProfileKeyIndex, UserProfileKeyType},
        },
    },
    identifiers::QualifiedUserName,
    messages::client_as_out::EncryptedUserProfileCtype,
};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, Encode, Sqlite, encode::IsNull, error::BoxDynError};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

pub mod display_name;
pub(crate) mod persistence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    pub user_name: QualifiedUserName,
    pub display_name: Option<DisplayName>,
    pub profile_picture: Option<Asset>,
}

impl From<IndexedUserProfile> for UserProfile {
    fn from(user_profile: IndexedUserProfile) -> Self {
        Self {
            user_name: user_profile.user_name,
            display_name: user_profile.display_name,
            profile_picture: user_profile.profile_picture,
        }
    }
}

/// A user profile contains information about a user, such as their display name
/// and profile picture.
#[derive(
    Debug, Clone, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub(crate) struct IndexedUserProfile {
    user_name: QualifiedUserName,
    decryption_key_index: UserProfileKeyIndex,
    display_name: Option<DisplayName>,
    profile_picture: Option<Asset>,
}

impl IndexedUserProfile {
    pub(crate) fn new(
        user_name: QualifiedUserName,
        decryption_key_index: UserProfileKeyIndex,
        display_name: Option<DisplayName>,
        profile_picture: Option<Asset>,
    ) -> Self {
        Self {
            user_name,
            decryption_key_index,
            display_name,
            profile_picture,
        }
    }

    pub(crate) fn decryption_key_index(&self) -> &UserProfileKeyIndex {
        &self.decryption_key_index
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

impl sqlx::Type<Sqlite> for Asset {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as sqlx::Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for Asset {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        match self {
            Asset::Value(value) => Encode::<Sqlite>::encode_by_ref(value, buf),
        }
    }
}

impl<'r> Decode<'r, Sqlite> for Asset {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        Decode::<Sqlite>::decode(value).map(Asset::Value)
    }
}

impl Asset {
    pub fn value(&self) -> Option<&[u8]> {
        match self {
            Asset::Value(value) => Some(value),
        }
    }
}

impl EarEncryptable<UserProfileKey, EncryptedUserProfileCtype> for IndexedUserProfile {}
impl EarDecryptable<UserProfileKey, EncryptedUserProfileCtype> for IndexedUserProfile {}

impl IndexDecryptable<UserProfileKeyType, EncryptedUserProfileCtype> for IndexedUserProfile {}
impl IndexEncryptable<UserProfileKeyType, EncryptedUserProfileCtype> for IndexedUserProfile {}
