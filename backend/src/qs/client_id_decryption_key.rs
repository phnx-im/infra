// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::crypto::{errors::KeyGenerationError, hpke::ClientIdDecryptionKey};

use super::errors::GenerateAndStoreError;

pub(super) struct StorableClientIdDecryptionKey(ClientIdDecryptionKey);

impl Deref for StorableClientIdDecryptionKey {
    type Target = ClientIdDecryptionKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StorableClientIdDecryptionKey {
    pub(super) async fn generate_and_store(
        connection: impl sqlx::PgExecutor<'_>,
    ) -> Result<Self, GenerateAndStoreError> {
        let decryption_key =
            ClientIdDecryptionKey::generate().map_err(|_| KeyGenerationError::KeypairGeneration)?;
        let key = Self(decryption_key);
        key.store(connection).await?;
        Ok(key)
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;
    use sqlx::PgExecutor;

    use crate::persistence::StorageError;

    use super::StorableClientIdDecryptionKey;

    impl StorableClientIdDecryptionKey {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO qs_decryption_key (decryption_key) VALUES ($1)",
                PhnxCodec::to_vec(&self.0)?
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<Self>, StorageError> {
            sqlx::query!("SELECT * FROM qs_decryption_key",)
                .fetch_optional(connection)
                .await?
                .map(|record| {
                    let decryption_key = PhnxCodec::from_slice(&record.decryption_key)?;
                    Ok(Self(decryption_key))
                })
                .transpose()
        }
    }
}
