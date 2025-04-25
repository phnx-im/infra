// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::marker::PhantomData;

use crate::{
    LibraryError,
    crypto::{
        ear::{
            AEAD_KEY_SIZE, EarDecryptable, EarEncryptable, EarKey,
            keys::{EncryptedUserProfileKey, EncryptedUserProfileKeyCtype, IdentityLinkWrapperKey},
        },
        errors::{DecryptionError, EncryptionError, RandomnessError},
        kdf::{KDF_KEY_SIZE, KdfDerivable, KdfKey},
        secrets::Secret,
    },
    identifiers::QualifiedUserName,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError,
    sqlite::SqliteTypeInfo,
};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use tracing::error;

/// Marker trait for indexed keys
pub trait IndexedKeyType {
    type DerivationContext<'a>: tls_codec::Serialize + Clone;

    // The `LABEL` constant is used to identify the key type in the database.
    const LABEL: &'static str;
}

/// Marker trait for keys that can be randomly generated
pub trait RandomlyGeneratable {}

// Dummy wrapper type to avoid orphan problem.
#[derive(Default)]
pub struct KeyTypeInstance<KT: IndexedKeyType>(PhantomData<KT>);

impl<'q, KT: IndexedKeyType> Encode<'q, Sqlite> for KeyTypeInstance<KT> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <&str as Encode<Sqlite>>::encode_by_ref(&KT::LABEL, buf)
    }
}

impl<'r, KT: IndexedKeyType> Decode<'r, Sqlite> for KeyTypeInstance<KT> {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let label: &str = Decode::<Sqlite>::decode(value)?;
        if label != KT::LABEL {
            return Err(BoxDynError::from(format!(
                "Invalid key type label: expected {}, got {}",
                KT::LABEL,
                label
            )));
        }
        Ok(Self(PhantomData))
    }
}

impl<KT: IndexedKeyType> Type<Sqlite> for KeyTypeInstance<KT> {
    fn type_info() -> SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<KT: IndexedKeyType> KeyTypeInstance<KT> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<KT: IndexedKeyType> tls_codec::Size for KeyTypeInstance<KT> {
    fn tls_serialized_len(&self) -> usize {
        KT::LABEL.len()
    }
}

impl<KT: IndexedKeyType> tls_codec::Serialize for KeyTypeInstance<KT> {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let label = KT::LABEL.as_bytes();
        let written = writer.write(label)?;
        Ok(written)
    }
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
#[serde(transparent)]
pub struct TypedSecret<KT, ST, const SIZE: usize> {
    secret: Secret<SIZE>,
    _type: PhantomData<(KT, ST)>,
}

impl<'q, KT, ST, const SIZE: usize> Encode<'q, Sqlite> for TypedSecret<KT, ST, SIZE> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        self.secret.encode_by_ref(buf)
    }
}

impl<KT, ST, const SIZE: usize> Type<Sqlite> for TypedSecret<KT, ST, SIZE> {
    fn type_info() -> SqliteTypeInfo {
        Secret::<SIZE>::type_info()
    }
}

impl<'r, KT, ST, const SIZE: usize> Decode<'r, Sqlite> for TypedSecret<KT, ST, SIZE> {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let secret: Secret<SIZE> = Decode::<Sqlite>::decode(value)?;
        Ok(Self {
            secret,
            _type: PhantomData,
        })
    }
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub struct BaseSecretType;
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub struct KeySecretType;
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub struct IndexSecretType;

pub type BaseSecret<KT> = TypedSecret<KT, BaseSecretType, KDF_KEY_SIZE>;
pub type Key<KT> = TypedSecret<KT, KeySecretType, AEAD_KEY_SIZE>;
pub type Index<KT> = TypedSecret<KT, IndexSecretType, AEAD_KEY_SIZE>;

impl<KT> BaseSecret<KT> {
    pub fn random() -> Result<Self, RandomnessError> {
        let value = Secret::<KDF_KEY_SIZE>::random()?;
        Ok(Self {
            secret: value,
            _type: PhantomData,
        })
    }
}

impl<KT> AsRef<Secret<KDF_KEY_SIZE>> for BaseSecret<KT> {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.secret
    }
}

impl<KT> KdfKey for BaseSecret<KT> {
    const ADDITIONAL_LABEL: &'static str = "indexed key base secret";
}

impl<KT, ST, const LENGTH: usize> From<Secret<LENGTH>> for TypedSecret<KT, ST, LENGTH> {
    fn from(value: Secret<LENGTH>) -> Self {
        Self {
            secret: value,
            _type: PhantomData,
        }
    }
}

impl<KT: RandomlyGeneratable> Key<KT> {
    pub fn random() -> Result<Self, RandomnessError> {
        let value = Secret::<AEAD_KEY_SIZE>::random()?;
        Ok(Self {
            secret: value,
            _type: PhantomData,
        })
    }
}

impl<KT> AsRef<Secret<AEAD_KEY_SIZE>> for Key<KT> {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.secret
    }
}

#[derive(TlsSerialize, TlsSize)]
struct DerivationContext<'a, KT: IndexedKeyType> {
    context: KT::DerivationContext<'a>,
    key_type_instance: KeyTypeInstance<KT>,
}

impl<KT: IndexedKeyType> KdfDerivable<BaseSecret<KT>, DerivationContext<'_, KT>, AEAD_KEY_SIZE>
    for Key<KT>
{
    const LABEL: &'static str = "key";
}

impl<KT: IndexedKeyType> KdfDerivable<BaseSecret<KT>, DerivationContext<'_, KT>, AEAD_KEY_SIZE>
    for Index<KT>
{
    const LABEL: &'static str = "index";
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub struct IndexedAeadKey<KT> {
    base_secret: BaseSecret<KT>,
    key: Key<KT>,
    index: Index<KT>,
}

impl<KT: IndexedKeyType> IndexedAeadKey<KT> {
    pub fn from_parts(base_secret: BaseSecret<KT>, key: Key<KT>, index: Index<KT>) -> Self {
        Self {
            base_secret,
            key,
            index,
        }
    }

    pub fn from_base_secret(
        base_secret: BaseSecret<KT>,
        context: KT::DerivationContext<'_>,
    ) -> Result<Self, LibraryError> {
        let derive_context = DerivationContext {
            context,
            key_type_instance: KeyTypeInstance::<KT>::new(),
        };
        let key =
            <Key<KT> as KdfDerivable<_, _, AEAD_KEY_SIZE>>::derive(&base_secret, &derive_context)?;
        let index = Index::derive(&base_secret, &derive_context)?;
        Ok(Self {
            base_secret,
            key,
            index,
        })
    }

    pub fn base_secret(&self) -> &BaseSecret<KT> {
        &self.base_secret
    }

    pub fn index(&self) -> &Index<KT> {
        &self.index
    }

    pub fn key(&self) -> &Key<KT> {
        &self.key
    }
}

impl<KT> AsRef<Secret<AEAD_KEY_SIZE>> for IndexedAeadKey<KT> {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key.secret
    }
}

impl<KT> EarKey for IndexedAeadKey<KT> {}

// User profile key

#[derive(
    Clone, Debug, Serialize, Deserialize, Eq, PartialEq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub struct UserProfileKeyType;

impl IndexedKeyType for UserProfileKeyType {
    type DerivationContext<'a> = &'a QualifiedUserName;

    const LABEL: &'static str = "user_profile_key";
}

pub type UserProfileKeyIndex = Index<UserProfileKeyType>;

pub type UserProfileBaseSecret = BaseSecret<UserProfileKeyType>;

pub type UserProfileKey = IndexedAeadKey<UserProfileKeyType>;

impl UserProfileKey {
    pub fn random(user_name: &QualifiedUserName) -> Result<Self, RandomnessError> {
        let base_secret = BaseSecret::random()?;
        Self::from_base_secret(base_secret, user_name).map_err(|e| {
            error!(error = %e, "Key derivation error");
            RandomnessError::InsufficientRandomness
        })
    }

    pub fn encrypt(
        &self,
        wrapper_key: &IdentityLinkWrapperKey,
        user_name: &QualifiedUserName,
    ) -> Result<EncryptedUserProfileKey, EncryptionError> {
        self.base_secret.encrypt_with_aad(wrapper_key, user_name)
    }

    pub fn decrypt(
        wrapper_key: &IdentityLinkWrapperKey,
        encrypted_key: &EncryptedUserProfileKey,
        user_name: &QualifiedUserName,
    ) -> Result<Self, DecryptionError> {
        let base_secret = BaseSecret::decrypt_with_aad(wrapper_key, encrypted_key, user_name)?;
        Self::from_base_secret(base_secret, user_name).map_err(|e| {
            error!(error = %e, "Key derivation error");
            DecryptionError::DecryptionError
        })
    }
}

impl EarEncryptable<IdentityLinkWrapperKey, EncryptedUserProfileKeyCtype>
    for UserProfileBaseSecret
{
}
impl EarDecryptable<IdentityLinkWrapperKey, EncryptedUserProfileKeyCtype>
    for UserProfileBaseSecret
{
}
