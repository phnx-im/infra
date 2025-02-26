// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use phnxtypes::codec::PhnxCodec;
use privacypass::{
    TruncatedTokenKeyId,
    batched_tokens_ristretto255::server::BatchedKeyStore,
    private_tokens::{Ristretto255, VoprfServer},
};
use sqlx::{Acquire, Postgres, Transaction};
use tokio::sync::Mutex;

pub(super) struct AuthServiceBatchedKeyStoreProvider<'a, 'b> {
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
    async fn insert(
        &self,
        truncated_token_key_id: TruncatedTokenKeyId,
        server: VoprfServer<Ristretto255>,
    ) {
        let Ok(server_bytes) = PhnxCodec::to_vec(&server) else {
            return;
        };
        let mut transaction = self.transaction_mutex.lock().await;
        let Ok(connection) = transaction.acquire().await else {
            return;
        };
        let _ = sqlx::query!(
            "INSERT INTO as_batched_keys (token_key_id, voprf_server) VALUES ($1, $2)",
            truncated_token_key_id as i16,
            server_bytes,
        )
        .execute(connection)
        .await;
    }

    /// Returns a keypair with a given `truncated_token_key_id` from the key store.
    async fn get(
        &self,
        truncated_token_key_id: &TruncatedTokenKeyId,
    ) -> Option<VoprfServer<Ristretto255>> {
        let mut transaction = self.transaction_mutex.lock().await;
        let connection = transaction.acquire().await.ok()?;
        let server_bytes_record = sqlx::query!(
            "SELECT voprf_server FROM as_batched_keys WHERE token_key_id = $1",
            *truncated_token_key_id as i16,
        )
        .fetch_one(connection)
        .await
        .ok()?;
        PhnxCodec::from_slice(&server_bytes_record.voprf_server).ok()
    }
}
