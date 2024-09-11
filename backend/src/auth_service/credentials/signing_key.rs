// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use mls_assist::openmls::prelude::SignatureScheme;
use phnxtypes::{
    credentials::{keys::AsSigningKey, AsCredential},
    identifiers::Fqdn,
};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection, PgExecutor};

use super::CredentialGenerationError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::auth_service) struct SigningKey(AsSigningKey);

impl Deref for SigningKey {
    type Target = AsSigningKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SigningKey {
    pub(in crate::auth_service) async fn generate_store_and_activate(
        connection: &mut PgConnection,
        domain: Fqdn,
        scheme: SignatureScheme,
    ) -> Result<Self, CredentialGenerationError> {
        let (_, signing_key) = AsCredential::new(scheme, domain, None)?;
        let signing_key = SigningKey(signing_key);
        let mut transaction = connection.begin().await?;
        signing_key.store(&mut *transaction).await?;
        signing_key.activate(&mut *transaction).await?;
        transaction.commit().await?;
        Ok(signing_key)
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;

    use crate::{auth_service::credentials::CredentialType, persistence::StorageError};

    use super::*;

    impl SigningKey {
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
                self.0.credential().fingerprint().as_bytes(),
                PhnxCodec::to_vec(&self)?,
                false,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<SigningKey>, StorageError> {
            sqlx::query!(
                "SELECT signing_key FROM as_signing_keys WHERE currently_active = true AND cred_type = $1",
                CredentialType::As as _
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                let signing_key = PhnxCodec::from_slice(&record.signing_key)?;
                Ok(SigningKey(signing_key))
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
                self.0.credential().fingerprint().as_bytes()
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }
}
