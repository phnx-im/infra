// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use sqlx::query;

use super::OwnClientInfo;

impl OwnClientInfo {
    pub(crate) async fn store(&self, executor: impl sqlx::SqliteExecutor<'_>) -> sqlx::Result<()> {
        let user_name = self.as_client_id.user_name();
        let client_id = self.as_client_id.client_id();
        query!(
            "INSERT INTO own_client_info (
                server_url,
                qs_user_id,
                qs_client_id,
                as_user_name,
                as_client_uuid
            ) VALUES (?, ?, ?, ?, ?)",
            self.server_url,
            self.qs_user_id,
            self.qs_client_id,
            user_name,
            client_id,
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use phnxtypes::identifiers::{AsClientId, QsClientId, QsUserId};
    use sqlx::{Row, SqlitePool};
    use uuid::Uuid;

    use super::*;

    #[sqlx::test]
    async fn store(pool: SqlitePool) -> anyhow::Result<()> {
        let own_client_info = OwnClientInfo {
            server_url: "https://localhost".to_string(),
            qs_user_id: QsUserId::random(),
            qs_client_id: QsClientId::random(&mut rand::thread_rng()),
            as_client_id: AsClientId::new("alice@localhost".parse()?, Uuid::new_v4()),
        };

        own_client_info.store(&pool).await?;

        let row = sqlx::query("SELECT * FROM own_client_info")
            .fetch_one(&pool)
            .await?;
        let server_url = row.try_get(0)?;
        let qs_user_id = row.try_get(1)?;
        let qs_client_id = row.try_get(2)?;
        let as_user_name = row.try_get(3)?;
        let as_client_uuid = row.try_get(4)?;
        let loaded = OwnClientInfo {
            server_url,
            qs_user_id,
            qs_client_id,
            as_client_id: AsClientId::new(as_user_name, as_client_uuid),
        };

        assert_eq!(loaded, own_client_info);

        Ok(())
    }
}
