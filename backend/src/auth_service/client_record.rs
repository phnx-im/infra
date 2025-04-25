// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    credentials::ClientCredential, crypto::RatchetEncryptionKey, identifiers::AsClientId,
    messages::client_as::AsQueueRatchet, time::TimeStamp,
};
use sqlx::{Connection, PgConnection};

use crate::errors::StorageError;

use super::queue::Queue;

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct ClientRecord {
    pub(super) queue_encryption_key: RatchetEncryptionKey,
    pub(super) ratchet_key: AsQueueRatchet,
    pub(super) activity_time: TimeStamp,
    pub(super) credential: ClientCredential,
    pub(super) token_allowance: i32,
}

const DEFAULT_TOKEN_ALLOWANCE: i32 = 1000;

impl ClientRecord {
    pub(super) async fn new_and_store(
        connection: &mut PgConnection,
        queue_encryption_key: RatchetEncryptionKey,
        ratchet_key: AsQueueRatchet,
        credential: ClientCredential,
    ) -> Result<Self, StorageError> {
        let record = Self {
            queue_encryption_key,
            ratchet_key,
            activity_time: TimeStamp::now(),
            credential,
            token_allowance: DEFAULT_TOKEN_ALLOWANCE,
        };

        // Initialize the client's queue.
        let mut transaction = connection.begin().await?;
        record.store(&mut *transaction).await?;
        Queue::new_and_store(record.client_id(), &mut *transaction).await?;
        transaction.commit().await?;

        Ok(record)
    }

    #[cfg(test)]
    pub(super) fn credential(&self) -> &ClientCredential {
        &self.credential
    }

    fn client_id(&self) -> &AsClientId {
        self.credential.identity()
    }
}

pub(crate) mod persistence {
    use phnxtypes::{
        codec::{BlobDecoded, BlobEncoded},
        credentials::persistence::FlatClientCredential,
        identifiers::QualifiedUserName,
    };
    use sqlx::{
        PgExecutor, query,
        types::chrono::{DateTime, Utc},
    };

    use super::*;

    impl ClientRecord {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            let activity_time = DateTime::<Utc>::from(self.activity_time);
            let client_credential = FlatClientCredential::from(&self.credential);
            let client_id = self.credential.identity();
            query!(
                "INSERT INTO as_client_records (
                    client_id,
                    user_name,
                    queue_encryption_key,
                    ratchet,
                    activity_time,
                    credential,
                    remaining_tokens
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                client_id.client_id(),
                client_id.user_name().to_string(),
                BlobEncoded(&self.queue_encryption_key) as _,
                BlobEncoded(&self.ratchet_key) as _,
                activity_time,
                client_credential as FlatClientCredential,
                self.token_allowance,
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn update(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            let activity_time = DateTime::<Utc>::from(self.activity_time);
            let client_credential = FlatClientCredential::from(&self.credential);
            let client_id = self.credential.identity();
            query!(
                "UPDATE as_client_records SET
                    queue_encryption_key = $1,
                    ratchet = $2,
                    activity_time = $3,
                    credential = $4,
                    remaining_tokens = $5
                WHERE client_id = $6",
                BlobEncoded(&self.queue_encryption_key) as _,
                BlobEncoded(&self.ratchet_key) as _,
                activity_time,
                client_credential as FlatClientCredential,
                self.token_allowance,
                client_id.client_id(),
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        pub(in crate::auth_service) async fn load(
            connection: impl PgExecutor<'_>,
            client_id: &AsClientId,
        ) -> Result<Option<ClientRecord>, StorageError> {
            query!(
                r#"SELECT
                    queue_encryption_key
                        AS "queue_encryption_key: BlobDecoded<RatchetEncryptionKey>",
                    ratchet AS "ratchet: BlobDecoded<AsQueueRatchet>",
                    activity_time,
                    credential AS "credential: FlatClientCredential",
                    remaining_tokens
                FROM as_client_records WHERE client_id = $1"#,
                client_id.client_id(),
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                Ok(ClientRecord {
                    queue_encryption_key: record.queue_encryption_key.into_inner(),
                    ratchet_key: record.ratchet.into_inner(),
                    activity_time: record.activity_time.into(),
                    credential: record.credential.into(),
                    token_allowance: record.remaining_tokens,
                })
            })
            .transpose()
        }

        pub(in crate::auth_service) async fn delete(
            connection: impl PgExecutor<'_>,
            client_id: &AsClientId,
        ) -> Result<(), StorageError> {
            query!(
                "DELETE FROM as_client_records WHERE client_id = $1",
                client_id.client_id(),
            )
            .execute(connection)
            .await?;
            Ok(())
        }

        /// Return the client credentials of a user for a given username.
        pub(in crate::auth_service) async fn load_user_credentials(
            connection: impl PgExecutor<'_>,
            user_name: &QualifiedUserName,
        ) -> Result<Vec<ClientCredential>, StorageError> {
            sqlx::query_scalar!(
                r#"SELECT credential as "client_credential: FlatClientCredential"
                FROM as_client_records WHERE user_name = $1"#,
                user_name.to_string(),
            )
            .fetch_all(connection)
            .await?
            .into_iter()
            .map(|flat_credential| {
                let client_credential = flat_credential.into();
                Ok(client_credential)
            })
            .collect()
        }
    }

    #[cfg(test)]
    pub(crate) mod tests {
        use mls_assist::openmls::prelude::SignatureScheme;
        use phnxtypes::{
            credentials::{ClientCredentialCsr, ClientCredentialPayload, CredentialFingerprint},
            crypto::{ratchet::QueueRatchet, signatures::signable::Signature},
            time::{Duration, ExpirationData},
        };
        use sqlx::PgPool;
        use uuid::Uuid;

        use crate::auth_service::user_record::persistence::tests::store_random_user_record;

        use super::*;

        pub(crate) async fn store_random_client_record(
            pool: &PgPool,
            client_id: AsClientId,
        ) -> anyhow::Result<ClientRecord> {
            let record = random_client_record(client_id)?;
            record.store(pool).await?;
            Ok(record)
        }

        fn random_client_record(client_id: AsClientId) -> Result<ClientRecord, anyhow::Error> {
            let (csr, _) = ClientCredentialCsr::new(client_id, SignatureScheme::ED25519)?;
            let expiration_data = ExpirationData::new(Duration::days(90));
            let record = ClientRecord {
                queue_encryption_key: RatchetEncryptionKey::from(b"encryption_key".to_vec()),
                ratchet_key: QueueRatchet::random()?,
                activity_time: TimeStamp::now(),
                credential: ClientCredential::new_for_test(
                    ClientCredentialPayload::new(
                        csr,
                        Some(expiration_data),
                        CredentialFingerprint::new_for_test(b"fingerprint".to_vec()),
                    ),
                    Signature::new_for_test(b"signature".to_vec()),
                ),
                token_allowance: 42,
            };
            Ok(record)
        }

        #[sqlx::test]
        async fn load(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(
                &pool,
                AsClientId::new(user_record.user_name().clone(), Uuid::new_v4()),
            )
            .await?;

            let loaded = ClientRecord::load(&pool, client_record.client_id())
                .await?
                .expect("missing client record");
            assert_eq!(loaded, client_record);

            Ok(())
        }

        #[sqlx::test]
        async fn load_user_credentials(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_records = vec![
                store_random_client_record(
                    &pool,
                    AsClientId::new(user_record.user_name().clone(), Uuid::new_v4()),
                )
                .await?,
                store_random_client_record(
                    &pool,
                    AsClientId::new(user_record.user_name().clone(), Uuid::new_v4()),
                )
                .await?,
                store_random_client_record(
                    &pool,
                    AsClientId::new(user_record.user_name().clone(), Uuid::new_v4()),
                )
                .await?,
            ];

            let mut loaded =
                ClientRecord::load_user_credentials(&pool, user_record.user_name()).await?;
            loaded.sort_by_key(|record| record.identity().client_id());
            let mut expected: Vec<_> = client_records
                .into_iter()
                .map(|record| record.credential)
                .collect();
            expected.sort_by_key(|credential| credential.identity().client_id());

            assert_eq!(loaded, expected);

            Ok(())
        }

        #[sqlx::test]
        async fn update(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(
                &pool,
                AsClientId::new(user_record.user_name().clone(), Uuid::new_v4()),
            )
            .await?;

            let loaded = ClientRecord::load(&pool, client_record.client_id())
                .await?
                .expect("missing client record");
            assert_eq!(loaded, client_record);

            let updated_client_record = random_client_record(client_record.client_id().clone())?;

            updated_client_record.update(&pool).await?;
            let loaded = ClientRecord::load(&pool, client_record.client_id())
                .await?
                .expect("missing client record");
            assert_eq!(loaded, updated_client_record);

            Ok(())
        }

        #[sqlx::test]
        async fn delete(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(
                &pool,
                AsClientId::new(user_record.user_name().clone(), Uuid::new_v4()),
            )
            .await?;

            let loaded = ClientRecord::load(&pool, client_record.client_id())
                .await?
                .expect("missing client record");
            assert_eq!(loaded, client_record);

            ClientRecord::delete(&pool, client_record.client_id()).await?;

            let loaded = ClientRecord::load(&pool, client_record.client_id()).await?;
            assert!(loaded.is_none());

            Ok(())
        }
    }
}
