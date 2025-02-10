// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use mls_assist::openmls::prelude::SignatureScheme;
use phnxtypes::{
    codec::persist::BlobPersist,
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

impl BlobPersist for StorableSigningKey {}

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
    use phnxtypes::codec::persist::BlobPersisted;

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
                self.persist() as _,
                false,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<AsSigningKey>, StorageError> {
            let value = sqlx::query_scalar!(
                r#"SELECT signing_key AS "signing_key: _"
                FROM as_signing_keys
                WHERE currently_active = true AND cred_type = $1"#,
                CredentialType::As as _
            )
            .fetch_optional(connection)
            .await?
            .map(|BlobPersisted(key): BlobPersisted<StorableSigningKey>| key.into());
            Ok(value)
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
            let credentials = sqlx::query_scalar!(
                r#"SELECT signing_key AS "signing_key: _"
                FROM as_signing_keys
                WHERE cred_type = $1"#,
                CredentialType::As as _
            )
            .fetch_all(connection)
            .await?
            .into_iter()
            .map(|BlobPersisted(key): BlobPersisted<StorableSigningKey>| {
                AsSigningKey::from(key).take_credential()
            })
            .collect();
            Ok(credentials)
        }
    }
}
