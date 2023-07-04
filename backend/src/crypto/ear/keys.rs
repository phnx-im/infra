// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs implementing the various keys for EAR
//! throughout the backend. Keys can either provide their own constructors or
//! implement the [`KdfDerivable`] trait to allow derivation from other key.

use mls_assist::openmls::prelude::GroupId;
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use utoipa::ToSchema;

use crate::crypto::{
    kdf::{
        keys::{InitialClientKdfKey, RatchetSecret, RosterKdfKey},
        KdfDerivable,
    },
    secrets::Secret,
    RandomnessError,
};

use super::{traits::EarKey, Ciphertext, EarDecryptable, EarEncryptable, AEAD_KEY_SIZE};

pub type GroupStateEarKeySecret = Secret<AEAD_KEY_SIZE>;

/// Key to encrypt/decrypt the roster of the DS group state. Roster keys can be
/// derived either from an initial client KDF key or from a derived roster KDF
/// key.
#[derive(Debug, Clone, TlsSerialize, TlsDeserializeBytes, TlsSize, Serialize, Deserialize)]
pub struct GroupStateEarKey {
    key: GroupStateEarKeySecret,
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

/// EAR key for the [`PushToken`] structs.
#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema, Serialize, Deserialize,
)]
pub struct PushTokenEarKey {
    key: PushTokenEarKeySecret,
}

impl PushTokenEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: AddPackageEarKeySecret::random()?,
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

pub type AddPackageEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`AddPackage`]s.
#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema, Serialize, Deserialize,
)]
pub struct AddPackageEarKey {
    key: AddPackageEarKeySecret,
}

impl AddPackageEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: AddPackageEarKeySecret::random()?,
        })
    }
}

impl EarKey for AddPackageEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for AddPackageEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for AddPackageEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type ClientCredentialEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`ClientCredential`]s.
#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema, Serialize, Deserialize,
)]
pub struct ClientCredentialEarKey {
    key: ClientCredentialEarKeySecret,
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
    Serialize, Deserialize, Clone, Debug, PartialEq, TlsSerialize, TlsDeserializeBytes, TlsSize,
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

pub type SignatureEarKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct SignatureEarKey {
    key: SignatureEarKeySecret,
}

impl SignatureEarKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: SignatureEarKeySecret::random()?,
        })
    }
}

impl EarKey for SignatureEarKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for SignatureEarKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for SignatureEarKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}

pub type WelcomeAttributionInfoEarKeySecret = Secret<AEAD_KEY_SIZE>;

// EAR key used to encrypt [`WelcomeAttributionInfo`]s.
#[derive(
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema, Serialize, Deserialize,
)]
pub struct WelcomeAttributionInfoEarKey {
    key: WelcomeAttributionInfoEarKeySecret,
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
    Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize, ToSchema, Serialize, Deserialize,
)]
pub struct FriendshipPackageEarKey {
    key: FriendshipPackageEarKeySecret,
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

impl EarEncryptable<SignatureEarKeyWrapperKey, EncryptedSignatureEarKey> for SignatureEarKey {}
impl EarDecryptable<SignatureEarKeyWrapperKey, EncryptedSignatureEarKey> for SignatureEarKey {}

#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsSize, TlsDeserializeBytes)]
pub struct EncryptedSignatureEarKey {
    ciphertext: Ciphertext,
}

impl From<Ciphertext> for EncryptedSignatureEarKey {
    fn from(ciphertext: Ciphertext) -> Self {
        Self { ciphertext }
    }
}

impl AsRef<Ciphertext> for EncryptedSignatureEarKey {
    fn as_ref(&self) -> &Ciphertext {
        &self.ciphertext
    }
}

pub type SignatureEarKeyWrapperKeySecret = Secret<AEAD_KEY_SIZE>;

#[derive(Serialize, Deserialize, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct SignatureEarKeyWrapperKey {
    key: SignatureEarKeyWrapperKeySecret,
}

impl SignatureEarKeyWrapperKey {
    pub fn random() -> Result<Self, RandomnessError> {
        Ok(Self {
            key: SignatureEarKeyWrapperKeySecret::random()?,
        })
    }
}

impl EarKey for SignatureEarKeyWrapperKey {}

impl AsRef<Secret<AEAD_KEY_SIZE>> for SignatureEarKeyWrapperKey {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key
    }
}

impl From<Secret<AEAD_KEY_SIZE>> for SignatureEarKeyWrapperKey {
    fn from(secret: Secret<AEAD_KEY_SIZE>) -> Self {
        Self { key: secret }
    }
}
