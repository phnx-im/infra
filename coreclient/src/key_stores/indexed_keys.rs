// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::marker::PhantomData;

use phnxtypes::crypto::{
    ear::{
        AEAD_KEY_SIZE, EarDecryptable, EarEncryptable, EarKey,
        keys::{EncryptedUserProfileKey, IdentityLinkWrapperKey},
    },
    errors::RandomnessError,
    kdf::{KDF_KEY_SIZE, KdfDerivable, KdfKey},
    secrets::Secret,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError, query, query_as,
    sqlite::SqliteTypeInfo,
};
use tracing::error;

trait KeyType {
    const LABEL: &'static str;
}

// Dummy wrapper type to avoid orphan problem.
struct KeyTypeInstance<KT: KeyType>(PhantomData<KT>);

impl<'q, KT: KeyType> Encode<'q, Sqlite> for KeyTypeInstance<KT> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <&str as Encode<Sqlite>>::encode_by_ref(&KT::LABEL, buf)
    }
}

impl<'r, KT: KeyType> Decode<'r, Sqlite> for KeyTypeInstance<KT> {
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

impl<KT: KeyType> Type<Sqlite> for KeyTypeInstance<KT> {
    fn type_info() -> SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<KT: KeyType> KeyTypeInstance<KT> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<KT: KeyType> tls_codec::Size for KeyTypeInstance<KT> {
    fn tls_serialized_len(&self) -> usize {
        KT::LABEL.as_bytes().len()
    }
}

impl<KT: KeyType> tls_codec::Serialize for KeyTypeInstance<KT> {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let label = KT::LABEL.as_bytes();
        let written = writer.write(label)?;
        Ok(written)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct TypedSecret<KT, ST, const SIZE: usize> {
    value: Secret<SIZE>,
    _type: PhantomData<(KT, ST)>,
}

impl<'q, KT, ST, const SIZE: usize> Encode<'q, Sqlite> for TypedSecret<KT, ST, SIZE> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        self.value.encode_by_ref(buf)
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
            value: secret,
            _type: PhantomData,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct BaseSecretType;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct KeySecretType;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct IndexSecretType;

type BaseSecret<KT> = TypedSecret<KT, BaseSecretType, KDF_KEY_SIZE>;
type Key<KT> = TypedSecret<KT, KeySecretType, AEAD_KEY_SIZE>;
type Index<KT> = TypedSecret<KT, IndexSecretType, AEAD_KEY_SIZE>;

impl<KT> BaseSecret<KT> {
    pub fn random() -> Result<Self, RandomnessError> {
        let value = Secret::<KDF_KEY_SIZE>::random()?;
        Ok(Self {
            value,
            _type: PhantomData,
        })
    }
}

impl<KT> AsRef<Secret<KDF_KEY_SIZE>> for BaseSecret<KT> {
    fn as_ref(&self) -> &Secret<KDF_KEY_SIZE> {
        &self.value
    }
}

impl<KT> KdfKey for BaseSecret<KT> {
    const ADDITIONAL_LABEL: &'static str = "indexed key base secret";
}

impl<KT, ST, const LENGTH: usize> From<Secret<LENGTH>> for TypedSecret<KT, ST, LENGTH> {
    fn from(value: Secret<LENGTH>) -> Self {
        Self {
            value,
            _type: PhantomData,
        }
    }
}

impl<KT, ST, const LENGTH: usize> TypedSecret<KT, ST, LENGTH> {}

impl<KT: KeyType> KdfDerivable<BaseSecret<KT>, KeyTypeInstance<KT>, AEAD_KEY_SIZE> for Key<KT> {
    const LABEL: &'static str = "key";
}

impl<KT: KeyType> KdfDerivable<BaseSecret<KT>, KeyTypeInstance<KT>, AEAD_KEY_SIZE> for Index<KT> {
    const LABEL: &'static str = "index";
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct IndexedAeadKey<KT> {
    base_secret: BaseSecret<KT>,
    key: Key<KT>,
    index: Index<KT>,
}

impl<'q, KT> Encode<'q, Sqlite> for IndexedAeadKey<KT> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        self.base_secret.encode_by_ref(buf)?;
        self.key.encode_by_ref(buf)?;
        self.index.encode_by_ref(buf)?;
        Ok(IsNull::No)
    }
}

impl<'r, KT> Decode<'r, Sqlite> for IndexedAeadKey<KT> {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let (base_secret, key, index) = sqlx::decode::Decode::<Sqlite>::decode(value)?;
        Ok(Self {
            base_secret,
            key,
            index,
        })
    }
}

impl<KT> AsRef<Secret<AEAD_KEY_SIZE>> for IndexedAeadKey<KT> {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key.value
    }
}

impl<KT> EarKey for IndexedAeadKey<KT> {}

impl<KT: KeyType + Send + Unpin> IndexedAeadKey<KT> {
    pub(crate) async fn store(
        &self,
        connection: impl sqlx::SqliteExecutor<'_>,
    ) -> Result<(), sqlx::Error> {
        query!(
            "INSERT OR IGNORE INTO indexed_keys (base_secret, key_value, key_index, key_type) VALUES ($1, $2, $3, $4)",
            self.base_secret,
            self.key,
            self.index,
            KT
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn load(
        connection: impl sqlx::SqliteExecutor<'_>,
    ) -> Result<Self, sqlx::Error> {
        query_as!(IndexedAeadKey, r#"SELECT base_secret AS "base_secret: _", key_value AS "key: _", key_index AS "index: _" FROM indexed_keys WHERE key_type = ? LIMIT 1"#, KT)
            .fetch_one(connection)
            .await
    }

    pub(crate) async fn delete(
        &self,
        connection: impl sqlx::SqliteExecutor<'_>,
    ) -> Result<(), sqlx::Error> {
        query!(
            "DELETE FROM indexed_keys WHERE key_index = $1",
            self.index.value
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

// User profile key

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
struct UserProfileKeyType;

impl KeyType for UserProfileKeyType {
    const LABEL: &'static str = "user profile key";
}

pub(crate) type UserProfileKey = IndexedAeadKey<UserProfileKeyType>;

impl UserProfileKey {
    pub fn random() -> Result<Self, RandomnessError> {
        let base_secret = BaseSecret::random()?;
        let key = Key::derive(&base_secret, KeyTypeInstance::new()).map_err(|e| {
            error!("Key derivation error: {:?}", e);
            RandomnessError::InsufficientRandomness
        })?;
        let index = Index::derive(&base_secret, KeyTypeInstance::new()).map_err(|e| {
            error!("Index derivation error: {:?}", e);
            RandomnessError::InsufficientRandomness
        })?;
        Ok(Self {
            base_secret,
            key,
            index,
        })
    }
}

impl EarEncryptable<IdentityLinkWrapperKey, EncryptedUserProfileKey> for UserProfileKey {}
impl EarDecryptable<IdentityLinkWrapperKey, EncryptedUserProfileKey> for UserProfileKey {}
