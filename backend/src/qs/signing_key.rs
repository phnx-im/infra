// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::crypto::signatures::{
    keys::QsVerifyingKey,
    private_keys::{generate_signature_keypair, PrivateKey},
    traits::SigningKey,
};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

use super::errors::GenerateAndStoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct QsSigningKey {
    signing_key: PrivateKey,
    verifiying_key: QsVerifyingKey,
}

impl QsSigningKey {
    pub(super) async fn generate_and_store(
        connection: impl PgExecutor<'_>,
    ) -> Result<Self, GenerateAndStoreError> {
        let (signing_key, verifying_key) = generate_signature_keypair()?;
        let key = Self {
            signing_key,
            verifiying_key: QsVerifyingKey::new(verifying_key),
        };
        key.store(connection).await?;
        Ok(key)
    }

    pub(super) fn verifying_key(&self) -> &QsVerifyingKey {
        &self.verifiying_key
    }
}

impl AsRef<PrivateKey> for QsSigningKey {
    fn as_ref(&self) -> &PrivateKey {
        &self.signing_key
    }
}

impl SigningKey for QsSigningKey {}

mod persistence {
    use phnxtypes::codec::PhnxCodec;
    use sqlx::PgExecutor;

    use crate::persistence::StorageError;

    use super::*;

    impl QsSigningKey {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO qs_signing_key (signing_key) VALUES ($1)",
                PhnxCodec::to_vec(self)?
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<Self>, StorageError> {
            sqlx::query!("SELECT signing_key FROM qs_signing_key")
                .fetch_optional(connection)
                .await?
                .map(|record| {
                    let signing_key = PhnxCodec::from_slice(&record.signing_key)?;
                    Ok(signing_key)
                })
                .transpose()
        }
    }
}
