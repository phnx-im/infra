// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use phnxtypes::codec::PhnxCodec;
use privacypass::{
    batched_tokens_ristretto255::server::BatchedKeyStore,
    private_tokens::{Ristretto255, VoprfServer},
    TruncatedTokenKeyId,
};
use sqlx::{Postgres, Transaction};
use tokio::sync::Mutex;
use tracing::error;

pub(super) struct AuthServiceBatchedKeyStoreProvider<'a, 'b> {
    // TODO: Replace with a pool?
    transaction_mutex: Mutex<&'b mut Transaction<'a, Postgres>>,
}

impl<'a, 'b> AuthServiceBatchedKeyStoreProvider<'a, 'b> {
    pub(super) fn new(transaction: &'b mut Transaction<'a, Postgres>) -> Self {
        Self {
            transaction_mutex: Mutex::new(transaction),
        }
    }
}

#[async_trait]
impl BatchedKeyStore for AuthServiceBatchedKeyStoreProvider<'_, '_> {
    /// Inserts a keypair with a given `truncated_token_key_id` into the key store.
    // TODO: What is the semantics on collision?
    async fn insert(
        &self,
        truncated_token_key_id: TruncatedTokenKeyId,
        server: VoprfServer<Ristretto255>,
    ) {
        let Ok(server_bytes) = PhnxCodec::to_vec(&server) else {
            return;
        };
        let mut transaction = self.transaction_mutex.lock().await;
        if let Err(error) = sqlx::query!(
            "INSERT INTO as_batched_keys (token_key_id, voprf_server) VALUES ($1, $2)",
            truncated_token_key_id as i16,
            server_bytes,
        )
        .execute(&mut ***transaction)
        .await
        {
            error!(%error, "Failed to insert key into DB");
        }
    }

    /// Returns a keypair with a given `truncated_token_key_id` from the key store.
    async fn get(
        &self,
        truncated_token_key_id: &TruncatedTokenKeyId,
    ) -> Option<VoprfServer<Ristretto255>> {
        let mut transaction = self.transaction_mutex.lock().await;
        let token_key_id: i16 = (*truncated_token_key_id).into();
        let voprf_server = sqlx::query_scalar!(
            "SELECT voprf_server FROM as_batched_keys WHERE token_key_id = $1",
            token_key_id
        )
        .fetch_one(&mut ***transaction)
        .await
        .ok()?;
        // TODO: deserialize without allocating the buffer
        PhnxCodec::from_slice(&voprf_server).ok()
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn insert_get(pool: PgPool) -> anyhow::Result<()> {
        let mut transaction = pool.begin().await?;
        let provider = AuthServiceBatchedKeyStoreProvider::new(&mut transaction);

        let mut rng = rand::thread_rng();

        let value = VoprfServer::new(&mut rng).unwrap();
        provider.insert(1, value.clone()).await;

        let loaded = provider.get(&1).await.unwrap();
        assert_eq!(loaded, value);

        let value = VoprfServer::new(&mut rng).unwrap();
        provider.insert(2, value.clone()).await;

        let loaded = provider.get(&2).await.unwrap();
        assert_eq!(loaded, value);

        Ok(())
    }
}
