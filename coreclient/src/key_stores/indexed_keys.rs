// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::marker::PhantomData;

use phnxtypes::{
    LibraryError,
    crypto::{
        ear::{
            AEAD_KEY_SIZE, EarDecryptable, EarEncryptable, EarKey,
            keys::{EncryptedUserProfileKey, IdentityLinkWrapperKey},
        },
        errors::{DecryptionError, EncryptionError, RandomnessError},
        kdf::{KDF_KEY_SIZE, KdfDerivable, KdfKey},
        secrets::Secret,
    },
    identifiers::QualifiedUserName,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Connection, Database, Decode, Encode, Sqlite, Type, encode::IsNull, error::BoxDynError,
    sqlite::SqliteTypeInfo,
};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};
use tracing::error;

// The `LABEL` constant is used to identify the key type in the database.
pub(crate) trait KeyType {
    type DerivationContext<'a>: tls_codec::Serialize + Clone;

    const LABEL: &'static str;
}

#[allow(dead_code)]
pub(crate) trait Deletable: KeyType {}

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
        KT::LABEL.len()
    }
}

impl<KT: KeyType> tls_codec::Serialize for KeyTypeInstance<KT> {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, tls_codec::Error> {
        let label = KT::LABEL.as_bytes();
        let written = writer.write(label)?;
        Ok(written)
    }
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub(crate) struct TypedSecret<KT, ST, const SIZE: usize> {
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

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub(crate) struct BaseSecretType;
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
struct KeySecretType;
#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub(crate) struct IndexSecretType;

pub(crate) type BaseSecret<KT> = TypedSecret<KT, BaseSecretType, KDF_KEY_SIZE>;
type Key<KT> = TypedSecret<KT, KeySecretType, AEAD_KEY_SIZE>;
pub(crate) type Index<KT> = TypedSecret<KT, IndexSecretType, AEAD_KEY_SIZE>;

impl<KT> BaseSecret<KT> {
    pub(crate) fn random() -> Result<Self, RandomnessError> {
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

#[derive(TlsSerialize, TlsSize)]
struct DerivationContext<'a, KT: KeyType> {
    context: KT::DerivationContext<'a>,
    key_type_instance: KeyTypeInstance<KT>,
}

impl<KT: KeyType> KdfDerivable<BaseSecret<KT>, DerivationContext<'_, KT>, AEAD_KEY_SIZE>
    for Key<KT>
{
    const LABEL: &'static str = "key";
}

impl<KT: KeyType> KdfDerivable<BaseSecret<KT>, DerivationContext<'_, KT>, AEAD_KEY_SIZE>
    for Index<KT>
{
    const LABEL: &'static str = "index";
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub(crate) struct IndexedAeadKey<KT> {
    base_secret: BaseSecret<KT>,
    key: Key<KT>,
    index: Index<KT>,
}

impl<KT: KeyType> IndexedAeadKey<KT> {
    pub(crate) fn from_base_secret(
        base_secret: BaseSecret<KT>,
        context: KT::DerivationContext<'_>,
    ) -> Result<Self, LibraryError> {
        let derive_context = DerivationContext {
            context,
            key_type_instance: KeyTypeInstance::<KT>::new(),
        };
        let key = Key::derive(&base_secret, &derive_context)?;
        let index = Index::derive(&base_secret, &derive_context)?;
        Ok(Self {
            base_secret,
            key,
            index,
        })
    }

    pub(crate) fn base_secret(&self) -> &BaseSecret<KT> {
        &self.base_secret
    }

    pub(crate) fn index(&self) -> &Index<KT> {
        &self.index
    }
}

impl<KT> AsRef<Secret<AEAD_KEY_SIZE>> for IndexedAeadKey<KT> {
    fn as_ref(&self) -> &Secret<AEAD_KEY_SIZE> {
        &self.key.value
    }
}

impl<KT> EarKey for IndexedAeadKey<KT> {}

// User profile key

#[derive(
    Clone, Debug, Serialize, Deserialize, Eq, PartialEq, TlsDeserializeBytes, TlsSerialize, TlsSize,
)]
pub(crate) struct UserProfileKeyType;

impl KeyType for UserProfileKeyType {
    type DerivationContext<'a> = &'a QualifiedUserName;

    const LABEL: &'static str = "user_profile_key";
}

pub(crate) type UserProfileKeyIndex = Index<UserProfileKeyType>;

pub(crate) type UserProfileBaseSecret = BaseSecret<UserProfileKeyType>;

pub(crate) type UserProfileKey = IndexedAeadKey<UserProfileKeyType>;

impl UserProfileKey {
    pub(crate) fn random(user_name: &QualifiedUserName) -> Result<Self, RandomnessError> {
        let base_secret = BaseSecret::random()?;
        Self::from_base_secret(base_secret, user_name).map_err(|e| {
            error!(error = %e, "Key derivation error");
            RandomnessError::InsufficientRandomness
        })
    }

    pub(crate) fn encrypt(
        &self,
        wrapper_key: &IdentityLinkWrapperKey,
        user_name: &QualifiedUserName,
    ) -> Result<EncryptedUserProfileKey, EncryptionError> {
        self.base_secret.encrypt_with_aad(wrapper_key, user_name)
    }

    pub(crate) fn decrypt(
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

impl EarEncryptable<IdentityLinkWrapperKey, EncryptedUserProfileKey> for UserProfileBaseSecret {}
impl EarDecryptable<IdentityLinkWrapperKey, EncryptedUserProfileKey> for UserProfileBaseSecret {}

mod persistence {
    use sqlx::{SqliteConnection, SqliteExecutor, query, query_as};

    use super::*;

    impl<KT: KeyType + Send + Unpin> IndexedAeadKey<KT> {
        pub(crate) async fn store(
            &self,
            connection: impl SqliteExecutor<'_>,
        ) -> Result<(), sqlx::Error> {
            query!(
                "INSERT OR IGNORE INTO indexed_keys (base_secret, key_value, key_index)
                VALUES ($1, $2, $3)",
                self.base_secret,
                self.key,
                self.index
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(crate) async fn store_own(
            &self,
            connection: &mut SqliteConnection,
        ) -> Result<(), sqlx::Error> {
            let key_type = KeyTypeInstance::<KT>::new();
            let mut transaction = connection.begin().await?;
            query!(
                "INSERT OR IGNORE INTO indexed_keys (base_secret, key_value, key_index)
                VALUES ($1, $2, $3)",
                self.base_secret,
                self.key,
                self.index
            )
            .execute(&mut *transaction)
            .await?;
            query!(
                "INSERT OR IGNORE INTO own_key_indices (key_index, key_type) VALUES ($1, $2)",
                self.index,
                key_type
            )
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
            Ok(())
        }

        pub(crate) async fn load(
            connection: impl SqliteExecutor<'_>,
            index: &UserProfileKeyIndex,
        ) -> Result<Self, sqlx::Error> {
            query_as!(
                IndexedAeadKey,
                r#"
                SELECT
                    base_secret AS "base_secret: _",
                    key_value AS "key: _",
                    key_index AS "index: _"
                FROM indexed_keys
                WHERE key_index = ?
                LIMIT 1"#,
                index,
            )
            .fetch_one(connection)
            .await
        }

        pub(crate) async fn load_own(
            connection: impl SqliteExecutor<'_>,
        ) -> Result<Self, sqlx::Error> {
            let key_type = KeyTypeInstance::<KT>::new();
            query_as!(
                IndexedAeadKey,
                r#"SELECT
                    ik.key_index as "index: _",
                    ik.key_value as "key: _",
                    ik.base_secret as "base_secret: _"
                FROM own_key_indices oki
                JOIN indexed_keys ik ON oki.key_index = ik.key_index
                WHERE oki.key_type = ?"#,
                key_type
            )
            .fetch_one(connection)
            .await
        }
    }

    impl<KT: Deletable + Send + Unpin> IndexedAeadKey<KT> {
        #[allow(dead_code)]
        pub(crate) async fn delete(
            &self,
            connection: impl SqliteExecutor<'_>,
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
}
