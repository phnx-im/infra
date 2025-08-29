// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use aircommon::crypto::{errors::KeyGenerationError, hpke::ClientIdDecryptionKey};

use super::errors::GenerateAndStoreError;

#[derive(sqlx::Type)]
#[sqlx(transparent)]
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
    use sqlx::PgExecutor;

    use crate::errors::StorageError;

    use super::StorableClientIdDecryptionKey;

    impl StorableClientIdDecryptionKey {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO qs_decryption_key (decryption_key) VALUES ($1)",
                self as &Self,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<Self>, StorageError> {
            sqlx::query_scalar!(r#"SELECT decryption_key as "dk: _" FROM qs_decryption_key"#)
                .fetch_optional(connection)
                .await
                .map_err(Into::into)
        }
    }

    #[cfg(test)]
    mod tests {
        use aircommon::crypto::hpke::ClientIdDecryptionKey;
        use sqlx::PgPool;

        use super::*;

        #[sqlx::test]
        async fn load(pool: PgPool) -> anyhow::Result<()> {
            let key = StorableClientIdDecryptionKey(ClientIdDecryptionKey::generate()?);
            key.store(&pool).await?;

            let loaded = StorableClientIdDecryptionKey::load(&pool)
                .await?
                .expect("missing decryption key");
            assert_eq!(loaded.0, key.0);

            Ok(())
        }
    }
}
