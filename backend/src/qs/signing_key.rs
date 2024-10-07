// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use phnxtypes::crypto::signatures::keys::QsSigningKey;
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

use super::errors::GenerateAndStoreError;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub(super) struct StorableQsSigningKey(QsSigningKey);

impl Deref for StorableQsSigningKey {
    type Target = QsSigningKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StorableQsSigningKey {
    pub(super) async fn generate_and_store(
        connection: impl PgExecutor<'_>,
    ) -> Result<Self, GenerateAndStoreError> {
        let key = Self(QsSigningKey::generate()?);
        key.store(connection).await?;
        Ok(key)
    }
}

mod persistence {
    use crate::errors::StorageError;

    use super::*;

    impl StorableQsSigningKey {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            sqlx::query!(
                "INSERT INTO qs_signing_key (signing_key) VALUES ($1)",
                self as &Self
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
        ) -> Result<Option<Self>, StorageError> {
            sqlx::query_scalar!(r#"SELECT signing_key as "sk: _" FROM qs_signing_key"#)
                .fetch_optional(connection)
                .await
                .map_err(StorageError::from)
        }
    }
}
