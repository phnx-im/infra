// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::{Connection, SqliteConnection};

pub trait ConnectionExt {
    async fn with_transaction<T: Send>(
        &mut self,
        f: impl AsyncFnOnce(&mut sqlx::SqliteTransaction<'_>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T>;
}

impl ConnectionExt for SqliteConnection {
    async fn with_transaction<T: Send>(
        &mut self,
        f: impl AsyncFnOnce(&mut sqlx::SqliteTransaction<'_>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut txn = self.begin_with("BEGIN IMMEDIATE").await?;
        let value = f(&mut txn).await?;
        txn.commit().await?;
        Ok(value)
    }
}
