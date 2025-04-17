// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs implementing the various keys for EAR
//! throughout the backend. Keys can either provide their own constructors or
//! implement the [`KdfDerivable`] trait to allow derivation from other key.

use mls_assist::openmls::prelude::GroupId;

use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::{
    credentials::pseudonymous_credentials::PseudonymousCredentialTbs,
    crypto::{
        errors::RandomnessError,
        kdf::{
            KdfDerivable,
            keys::{ConnectionKey, InitialClientKdfKey, RatchetSecret, RosterKdfKey},
        },
        secrets::Secret,
    },
};

use super::{AEAD_KEY_SIZE, Ciphertext, EarDecryptable, EarEncryptable, traits::EarKey};

pub type GroupStateEarKeySecret = Secret<AEAD_KEY_SIZE>;

/// Key to encrypt/decrypt the roster of the DS group state. Roster keys can be
/// derived either from an initial client KDF key or from a derived roster KDF
/// key.
#[derive(Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct GroupStateEarKey {
    key: GroupStateEarKeySecret,
}

impl Type<Sqlite> for GroupStateEarKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for GroupStateEarKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(&self.key, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for GroupStateEarKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl GroupStateEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: GroupStateEarKeySecret::random()?,
        })
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for GroupStateEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl AsRef<Secret<AEAD_KEY_SIZE>> for GroupStateEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl EarKey for GroupStateEarKey {}

impl KdfDerivable<InitialClientKdfKey, GroupId, AEAD_KEY_SIZE> for GroupStateEarKey {
    const LABEL: &'static str = "roster ear key";
}

impl KdfDerivable<RosterKdfKey, GroupId, AEAD_KEY_SIZE> for GroupStateEarKey {
    const LABEL: &'static str = "roster kdf key";
}

pub type DeleteAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type PushTokenEarKeySecret = Secret<AEAD_KEY_SIZE>;

pub type RatchetKeySecret = Secret<AEAD_KEY_SIZE>;

/// EAR key for the [`crate::messages::push_token::PushToken`] structs.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct PushTokenEarKey {
    key: PushTokenEarKeySecret,
}

impl PushTokenEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: KeyPackageEarKeySecret::random()?,
        })
    }
}

impl EarKey for PushTokenEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for PushTokenEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for PushTokenEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type KeyPackageEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`AddPackage`]s.
#[derive(
    Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub struct KeyPackageEarKey {
    key: KeyPackageEarKeySecret,
}

impl Type<Sqlite> for KeyPackageEarKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&[u8] as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for KeyPackageEarKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(self.as_ref(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for KeyPackageEarKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl KeyPackageEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: KeyPackageEarKeySecret::random()?,
        })
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(key: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key }
    }
}

impl EarKey for KeyPackageEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for KeyPackageEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for KeyPackageEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type ClientCredentialEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`ClientCredential`]s.
#[derive(Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct ClientCredentialEarKey {
    key: ClientCredentialEarKeySecret,
}

impl Type<Sqlite> for ClientCredentialEarKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&[u8] as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ClientCredentialEarKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(self.as_ref(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ClientCredentialEarKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl ClientCredentialEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: ClientCredentialEarKeySecret::random()?,
        })
    }
}

impl EarKey for ClientCredentialEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for ClientCredentialEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for ClientCredentialEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type EnqueueAuthKeyEarKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct RatchetKey {
    key: RatchetKeySecret,
}

impl EarKey for RatchetKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for RatchetKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for RatchetKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl KdfDerivable<RatchetSecret, Vec<u8>, AEAD_KEY_SIZE> for RatchetKey {
    const LABEL: &'static str = "RatchetKey";
}

pub type IdentityLinkKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct IdentityLinkKey {
    key: IdentityLinkKeySecret,
}

impl Type<Sqlite> for IdentityLinkKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for IdentityLinkKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(&self.key, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for IdentityLinkKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl EarKey for IdentityLinkKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for IdentityLinkKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for IdentityLinkKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl KdfDerivable<ConnectionKey, PseudonymousCredentialTbs, AEAD_KEY_SIZE> for IdentityLinkKey {
    const LABEL: &'static str = "IdentityLinkKey";
}

pub type WelcomeAttributionInfoEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`WelcomeAttributionInfo`]s.
#[derive(
    Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub struct WelcomeAttributionInfoEarKey {
    key: WelcomeAttributionInfoEarKeySecret,
}

impl Type<Sqlite> for WelcomeAttributionInfoEarKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&[u8] as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for WelcomeAttributionInfoEarKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(self.as_ref(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for WelcomeAttributionInfoEarKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl WelcomeAttributionInfoEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: WelcomeAttributionInfoEarKeySecret::random()?,
        })
    }
}

impl EarKey for WelcomeAttributionInfoEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for WelcomeAttributionInfoEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for WelcomeAttributionInfoEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type FriendshipPackageEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`WelcomeAttributionInfo`]s.
#[derive(
    Clone, Debug, PartialEq, Eq, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize,
)]
pub struct FriendshipPackageEarKey {
    key: FriendshipPackageEarKeySecret,
}

impl Type<Sqlite> for FriendshipPackageEarKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <&[u8] as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for FriendshipPackageEarKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(self.as_ref(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for FriendshipPackageEarKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl FriendshipPackageEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: FriendshipPackageEarKeySecret::random()?,
        })
    }
}

impl EarKey for FriendshipPackageEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for FriendshipPackageEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for FriendshipPackageEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

impl EarEncryptable<IdentityLinkWrapperKey, EncryptedIdentityLinkKey> for IdentityLinkKey {}
impl EarDecryptable<IdentityLinkWrapperKey, EncryptedIdentityLinkKey> for IdentityLinkKey {}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes)]
pub struct EncryptedIdentityLinkKey {
    ciphertext: Ciphertext,
}

impl From<Ciphertext> for EncryptedIdentityLinkKey {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl From<EncryptedIdentityLinkKey> for Ciphertext {
    fn from(EncryptedIdentityLinkKey { ciphertext }: EncryptedIdentityLinkKey) -> Self {
        ciphertext
    }
}

impl AsRef<Ciphertext> for EncryptedIdentityLinkKey {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

pub type IdentityLinkWrapperKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct IdentityLinkWrapperKey {
    key: IdentityLinkWrapperKeySecret,
}

impl Type<Sqlite> for IdentityLinkWrapperKey {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <IdentityLinkWrapperKeySecret as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for IdentityLinkWrapperKey {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        Encode::<Sqlite>::encode_by_ref(&self.key, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for IdentityLinkWrapperKey {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let key = Decode::<Sqlite>::decode(value)?;
        Ok(Self { key })
    }
}

impl IdentityLinkWrapperKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: IdentityLinkWrapperKeySecret::random()?,
        })
    }
}

impl EarKey for IdentityLinkWrapperKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for IdentityLinkWrapperKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for IdentityLinkWrapperKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes)]
pub struct EncryptedUserProfileKey {
    ciphertext: Ciphertext,
}

impl From<Ciphertext> for EncryptedUserProfileKey {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl From<EncryptedUserProfileKey> for Ciphertext {
    fn from(EncryptedUserProfileKey { ciphertext }: EncryptedUserProfileKey) -> Self {
        ciphertext
    }
}

impl AsRef<Ciphertext> for EncryptedUserProfileKey {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}
