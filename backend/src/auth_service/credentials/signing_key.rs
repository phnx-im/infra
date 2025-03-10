// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use mls_assist::openmls::prelude::SignatureScheme;
use phnxtypes::{
    credentials::{keys::AsSigningKey, AsCredential, CredentialFingerprint},
    identifiers::Fqdn,
};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection, PgExecutor};

use super::CredentialGenerationError;

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::auth_service) enum StorableSigningKey {
    V1(AsSigningKey),
}

impl From<StorableSigningKey> for AsSigningKey {
    fn from(signing_key: StorableSigningKey) -> Self {
        match signing_key {
            StorableSigningKey::V1(signing_key) => signing_key,
        }
    }
}

impl From<AsSigningKey> for StorableSigningKey {
    fn from(signing_key: AsSigningKey) -> Self {
        StorableSigningKey::V1(signing_key)
    }
}

pub(in crate::auth_service) struct Credential(AsCredential);

impl Deref for Credential {
    type Target = AsCredential;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StorableSigningKey {
    pub(in crate::auth_service) async fn generate_store_and_activate(
        connection: &mut PgConnection,
        domain: Fqdn,
        scheme: SignatureScheme,
    ) -> Result<Self, CredentialGenerationError> {
        let (_, signing_key) = AsCredential::new(scheme, domain, None)?;
        let signing_key = StorableSigningKey::V1(signing_key);
        let mut transaction = connection.begin().await?;
        signing_key.store(&mut *transaction).await?;
        signing_key.activate(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(signing_key)
    }

    fn fingerprint(&self) -> &CredentialFingerprint {
        match self {
            StorableSigningKey::V1(signing_key) => signing_key.credential().fingerprint(),
        }
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;

    use crate::{auth_service::credentials::CredentialType, errors::StorageError};

    use super::*;

    impl StorableSigningKey {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO
                    as_signing_keys
                    (cred_type, credential_fingerprint, signing_key, currently_active)
                VALUES
                    ($1, $2, $3, $4)",
                CredentialType::As as _,
                self.fingerprint().as_bytes(),
                PhnxCodec::to_vec(&self)?,
                false,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<AsSigningKey>, StorageError> {
            sqlx::query!(
                "SELECT signing_key FROM as_signing_keys WHERE currently_active = true AND cred_type = $1",
                CredentialType::As as _
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                let signing_key: StorableSigningKey = PhnxCodec::from_slice(&record.signing_key)?;
                Ok(signing_key.into())
            })
            .transpose()
        }

        pub(super) async fn activate(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "UPDATE as_signing_keys
                SET currently_active = CASE
                    WHEN credential_fingerprint = $1 THEN true
                    ELSE false
                END
                WHERE currently_active = true OR credential_fingerprint = $1",
                self.fingerprint().as_bytes()
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }

    impl Credential {
        pub(in crate::auth_service) async fn load_all(
            connection: impl PgExecutor<'_>,
        ) -> Result<Vec<AsCredential>, StorageError> {
            let records = sqlx::query!(
                "SELECT signing_key FROM as_signing_keys WHERE cred_type = $1",
                CredentialType::As as _
            )
            .fetch_all(connection)
            .await?;

            let credentials = records
                .into_iter()
                .map(|record| {
                    let signing_key: StorableSigningKey =
                        PhnxCodec::from_slice(&record.signing_key)?;
                    let as_signing_key = AsSigningKey::from(signing_key);
                    Ok(as_signing_key.credential().clone())
                })
                .collect::<Result<Vec<_>, StorageError>>()?;

            Ok(credentials)
        }
    }

    #[cfg(test)]
    mod tests {
        use std::collections::HashSet;

        use phnxtypes::time::{Duration, ExpirationData};
        use sqlx::PgPool;

        use super::*;

        async fn store_random_signing_key(pool: &PgPool) -> anyhow::Result<StorableSigningKey> {
            let (_, key) = AsCredential::new(
                SignatureScheme::ED25519,
                "example.com".parse()?,
                Some(ExpirationData::new(Duration::days(42))),
            )?;
            let storable: StorableSigningKey = key.into();
            storable.store(pool).await?;
            Ok(storable)
        }

        fn comparable<T: Serialize>(value: &T) -> String {
            serde_json::to_string(value).unwrap()
        }

        #[sqlx::test]
        async fn load(pool: PgPool) -> anyhow::Result<()> {
            let key = store_random_signing_key(&pool).await?;

            let loaded = StorableSigningKey::load(&pool).await?;
            assert!(loaded.is_none()); // not active

            key.activate(&pool).await?;
            let loaded = StorableSigningKey::load(&pool)
                .await?
                .expect("missing signing key");
            assert_eq!(
                comparable::<StorableSigningKey>(&loaded.into()),
                comparable(&key)
            );

            Ok(())
        }

        #[sqlx::test]
        async fn activate(pool: PgPool) -> anyhow::Result<()> {
            let keys = [
                store_random_signing_key(&pool).await?,
                store_random_signing_key(&pool).await?,
                store_random_signing_key(&pool).await?,
            ];

            let loaded = StorableSigningKey::load(&pool).await?;
            assert!(loaded.is_none()); // not active

            keys[0].activate(&pool).await?;
            let loaded = StorableSigningKey::load(&pool)
                .await?
                .expect("missing signing key");
            assert_eq!(
                comparable::<StorableSigningKey>(&loaded.into()),
                comparable(&keys[0])
            );

            keys[1].activate(&pool).await?;
            let loaded = StorableSigningKey::load(&pool)
                .await?
                .expect("missing signing key");
            assert_eq!(
                comparable::<StorableSigningKey>(&loaded.into()),
                comparable(&keys[1])
            );

            keys[2].activate(&pool).await?;
            let loaded = StorableSigningKey::load(&pool)
                .await?
                .expect("missing signing key");
            assert_eq!(
                comparable::<StorableSigningKey>(&loaded.into()),
                comparable(&keys[2])
            );

            Ok(())
        }

        #[sqlx::test]
        async fn load_all(pool: PgPool) -> anyhow::Result<()> {
            let keys = [
                store_random_signing_key(&pool).await?,
                store_random_signing_key(&pool).await?,
                store_random_signing_key(&pool).await?,
            ];

            let loaded = Credential::load_all(&pool).await?;
            assert_eq!(loaded.len(), 3);

            let loaded_comparable: HashSet<_> = loaded.iter().map(comparable).collect();
            let expected_comparable: HashSet<_> = keys
                .into_iter()
                .map(AsSigningKey::from)
                .map(|value| comparable(value.credential()))
                .collect();
            assert_eq!(loaded_comparable, expected_comparable);

            Ok(())
        }
    }
}
