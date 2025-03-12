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
use sqlx::{
    encode::IsNull, error::BoxDynError, postgres::PgTypeInfo, Database, Decode, Encode,
    PgConnection, Postgres, Type,
};
use tokio::sync::Mutex;
use tracing::error;

pub(super) struct AuthServiceBatchedKeyStoreProvider<'a> {
    // Note: PgPool is not used here, because we need to use this provider in a transactional
    // context.
    connection: Mutex<&'a mut PgConnection>,
}

impl<'a> AuthServiceBatchedKeyStoreProvider<'a> {
    pub(super) fn new(connection: &'a mut PgConnection) -> Self {
        Self {
            connection: connection.into(),
        }
    }
}

struct StorableVorpfServer(VoprfServer<Ristretto255>);

impl Type<Postgres> for StorableVorpfServer {
    fn type_info() -> PgTypeInfo {
        <Vec<u8> as Type<Postgres>>::type_info()
    }
}

impl Encode<'_, Postgres> for StorableVorpfServer {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as Database>::ArgumentBuffer<'_>,
    ) -> Result<IsNull, BoxDynError> {
        PhnxCodec::to_vec(&self.0)?.encode(buf)
    }
}

impl Decode<'_, Postgres> for StorableVorpfServer {
    fn decode(value: <Postgres as Database>::ValueRef<'_>) -> Result<Self, BoxDynError> {
        let bytes: &[u8] = Decode::<Postgres>::decode(value)?;
        Ok(StorableVorpfServer(PhnxCodec::from_slice(bytes)?))
    }
}

#[async_trait]
impl BatchedKeyStore for AuthServiceBatchedKeyStoreProvider<'_> {
    /// Inserts a keypair with a given `truncated_token_key_id` into the key store.
    ///
    /// On conflict, an error is logged and the value is not inserted.
    async fn insert(
        &self,
        truncated_token_key_id: TruncatedTokenKeyId,
        server: VoprfServer<Ristretto255>,
    ) {
        let server = StorableVorpfServer(server);
        if let Err(error) = sqlx::query!(
            "INSERT INTO as_batched_keys (token_key_id, voprf_server)
            VALUES ($1, $2)",
            truncated_token_key_id as i16,
            server as StorableVorpfServer
        )
        .execute(&mut **self.connection.lock().await)
        .await
        {
            error!(%error, "Failed to insert key into batched key store");
        }
    }

    /// Returns a keypair with a given `truncated_token_key_id` from the key store.
    async fn get(
        &self,
        truncated_token_key_id: &TruncatedTokenKeyId,
    ) -> Option<VoprfServer<Ristretto255>> {
        let token_key_id: i16 = (*truncated_token_key_id).into();
        sqlx::query_scalar!(
            r#"SELECT voprf_server AS "voprf_server: StorableVorpfServer"
            FROM as_batched_keys
            WHERE token_key_id = $1"#,
            token_key_id
        )
        .fetch_optional(&mut **self.connection.lock().await)
        .await
        .inspect_err(|error| error!(%error, "Failed to fetch key from batched key store"))
        .ok()?
        .map(|StorableVorpfServer(voprf_server)| voprf_server)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn insert_get(pool: PgPool) -> anyhow::Result<()> {
        let mut connection = pool.acquire().await?;
        let provider = AuthServiceBatchedKeyStoreProvider::new(&mut connection);

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

    #[sqlx::test]
    async fn no_insert_on_conflict(pool: PgPool) -> anyhow::Result<()> {
        let mut connection = pool.acquire().await?;
        let provider = AuthServiceBatchedKeyStoreProvider::new(&mut connection);

        let mut rng = rand::thread_rng();

        let value_a = VoprfServer::new(&mut rng).unwrap();
        provider.insert(1, value_a.clone()).await;

        let loaded = provider.get(&1).await.unwrap();
        assert_eq!(loaded, value_a);

        let value_b = VoprfServer::new(&mut rng).unwrap();
        provider.insert(1, value_b.clone()).await;

        let loaded = provider.get(&1).await.unwrap();
        assert_eq!(loaded, value_a);

        Ok(())
    }
}
