// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module provides structs and functions to interact with users in the
//! various groups an InfraClient is a member of.

use display_name::BaseDisplayName;
pub use display_name::{DisplayName, DisplayNameError};
use phnxtypes::{
    LibraryError,
    crypto::{
        ear::{EarDecryptable, EarEncryptable},
        indexed_aead::{
            ciphertexts::{IndexDecryptable, IndexEncryptable},
            keys::{UserProfileKey, UserProfileKeyIndex, UserProfileKeyType},
        },
        signatures::{
            private_keys::SignatureVerificationError,
            signable::{Signable, Signature, SignedStruct, Verifiable, VerifiedStruct},
        },
    },
    identifiers::QualifiedUserName,
    messages::client_as_out::EncryptedUserProfileCtype,
};
use sealed::Seal;
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, Encode, Sqlite, encode::IsNull, error::BoxDynError};
use thiserror::Error;
use tls_codec::{Serialize as _, TlsDeserializeBytes, TlsSerialize, TlsSize};
use tracing::info;

pub mod display_name;
pub(crate) mod generate;
pub(crate) mod persistence;
pub(crate) mod process;
pub(crate) mod update;

const USER_PROFILE_LABEL: &str = "UserProfile";

impl Signable for IndexedUserProfile {
    type SignedOutput = SignedUserProfile;

    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tls_serialize_detached()
    }

    fn label(&self) -> &str {
        USER_PROFILE_LABEL
    }
}

impl SignedStruct<IndexedUserProfile> for SignedUserProfile {
    fn from_payload(payload: IndexedUserProfile, signature: Signature) -> Self {
        Self {
            tbs: payload,
            signature,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct SignedUserProfile {
    tbs: IndexedUserProfile,
    signature: Signature,
}

impl Verifiable for VerifiableUserProfile {
    fn unsigned_payload(&self) -> Result<Vec<u8>, tls_codec::Error> {
        self.tbs.tls_serialize_detached()
    }

    fn signature(&self) -> impl AsRef<[u8]> {
        &self.signature
    }

    fn label(&self) -> &str {
        USER_PROFILE_LABEL
    }
}

mod sealed {
    #[derive(Default)]
    pub struct Seal;
}

impl VerifiedStruct<VerifiableUserProfile> for UnvalidatedUserProfile {
    type SealingType = Seal;

    fn from_verifiable(verifiable: VerifiableUserProfile, _seal: Self::SealingType) -> Self {
        verifiable.tbs
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VerifiableUserProfile {
    tbs: UnvalidatedUserProfile,
    signature: Signature,
}

#[derive(Debug, Error)]
pub enum UserProfileValidationError {
    #[error("User profile is outdated")]
    OutdatedUserProfile {
        user_name: QualifiedUserName,
        epoch: u64,
    },
    #[error("Mismatching user name")]
    MismatchingUserName {
        expected: QualifiedUserName,
        actual: QualifiedUserName,
    },
    #[error(transparent)]
    InvalidSignature(#[from] SignatureVerificationError),
    #[error(transparent)]
    LibraryError(#[from] LibraryError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    pub user_name: QualifiedUserName,
    pub display_name: DisplayName,
    pub profile_picture: Option<Asset>,
}

impl UserProfile {
    pub fn from_user_name(user_name: &QualifiedUserName) -> Self {
        Self {
            user_name: user_name.clone(),
            display_name: DisplayName::from_user_name(user_name),
            profile_picture: None,
        }
    }
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
pub(crate) struct BaseIndexedUserProfile<const VALIDATED: bool> {
    user_name: QualifiedUserName,
    epoch: u64,
    decryption_key_index: UserProfileKeyIndex,
    display_name: BaseDisplayName<VALIDATED>,
    profile_picture: Option<Asset>,
}

pub(crate) type IndexedUserProfile = BaseIndexedUserProfile<true>;

pub(crate) type UnvalidatedUserProfile = BaseIndexedUserProfile<false>;

impl UnvalidatedUserProfile {
    /// Validates the display name and returns an [`IndexedUserProfile`].
    /// If the display name is invalid, it is replaced with a default
    /// based on the user name.
    pub fn validate_display_name(self) -> IndexedUserProfile {
        let display_name = self.display_name.validate().unwrap_or_else(|e| {
            info!(error = %e, "Invalid display name, generating default");
            DisplayName::from_user_name(&self.user_name)
        });
        IndexedUserProfile {
            user_name: self.user_name,
            epoch: self.epoch,
            decryption_key_index: self.decryption_key_index,
            display_name,
            profile_picture: self.profile_picture,
        }
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

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub(crate) struct EncryptableUserProfile(SignedUserProfile);

impl EarEncryptable<UserProfileKey, EncryptedUserProfileCtype> for EncryptableUserProfile {}
impl EarDecryptable<UserProfileKey, EncryptedUserProfileCtype> for VerifiableUserProfile {}

impl IndexEncryptable<UserProfileKeyType, EncryptedUserProfileCtype> for EncryptableUserProfile {}
impl IndexDecryptable<UserProfileKeyType, EncryptedUserProfileCtype> for VerifiableUserProfile {}
