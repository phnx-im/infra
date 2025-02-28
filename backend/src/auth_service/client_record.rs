// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    credentials::ClientCredential,
    crypto::{ratchet::QueueRatchet, RatchetEncryptionKey},
    identifiers::AsClientId,
    messages::{client_as::AsQueueMessagePayload, EncryptedAsQueueMessage},
    time::TimeStamp,
};
use sqlx::{Connection, PgConnection};

use crate::errors::StorageError;

use super::queue::Queue;

#[derive(Debug, Clone)]
pub(super) struct ClientRecord {
    pub(super) queue_encryption_key: RatchetEncryptionKey,
    pub(super) ratchet_key: QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>,
    pub(super) activity_time: TimeStamp,
    pub(super) credential: ClientCredential,
    pub(super) token_allowance: i32,
}

const DEFAULT_TOKEN_ALLOWANCE: i32 = 1000;

impl ClientRecord {
    pub(super) async fn new_and_store(
        connection: &mut PgConnection,
        queue_encryption_key: RatchetEncryptionKey,
        ratchet_key: QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>,
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
        record.store(&mut transaction).await?;
        Queue::new_and_store(record.client_id(), &mut transaction).await?;
        transaction.commit().await?;

        Ok(record)
    }

    fn client_id(&self) -> AsClientId {
        self.credential.identity()
    }
}

mod persistence {
    use phnxtypes::{
        codec::persist::{BlobPersist, BlobPersisted},
        credentials::persistence::FlatClientCredential,
        identifiers::QualifiedUserName,
    };
    use sqlx::{
        types::chrono::{DateTime, Utc},
        PgExecutor,
    };

    use super::*;

    impl ClientRecord {
        pub(super) async fn store(
            &self,
            connection: &mut PgConnection,
        ) -> Result<(), StorageError> {
            let client_credential = FlatClientCredential::from(self.credential.clone());
            let client_id = self.credential.identity();
            sqlx::query!(
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
                self.queue_encryption_key.persisting() as _,
                self.ratchet_key.persisting() as _,
                self.activity_time as TimeStamp,
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
            let client_credential = FlatClientCredential::from(self.credential.clone());
            let client_id = self.credential.identity();
            sqlx::query!(
                r#"UPDATE as_client_records SET
                    queue_encryption_key = $1,
                    ratchet = $2,
                    activity_time = $3,
                    credential = $4,
                    remaining_tokens = $5
                WHERE client_id = $6
                "#,
                self.queue_encryption_key.persisting() as _,
                self.ratchet_key.persisting() as _,
                self.activity_time as TimeStamp,
                client_credential as _,
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
            struct SqlClientRecord {
                queue_encryption_key: BlobPersisted<RatchetEncryptionKey>,
                ratchet:
                    BlobPersisted<QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>>,
                activity_time: DateTime<Utc>,
                credential: FlatClientCredential,
                remaining_tokens: i32,
            }

            sqlx::query_as!(
                SqlClientRecord,
                r#"SELECT
                    queue_encryption_key AS "queue_encryption_key: _",
                    ratchet AS "ratchet: _",
                    activity_time,
                    credential AS "credential: _",
                    remaining_tokens
                FROM as_client_records WHERE client_id = $1"#,
                client_id.client_id(),
            )
            .fetch_optional(connection)
            .await?
            .map(
                |SqlClientRecord {
                     queue_encryption_key: BlobPersisted(queue_encryption_key),
                     ratchet: BlobPersisted(ratchet_key),
                     activity_time,
                     credential,
                     remaining_tokens,
                 }: SqlClientRecord| {
                    Ok(ClientRecord {
                        queue_encryption_key,
                        ratchet_key,
                        activity_time: activity_time.into(),
                        credential: credential.into(),
                        token_allowance: remaining_tokens,
                    })
                },
            )
            .transpose()
        }

        pub(in crate::auth_service) async fn delete(
            connection: impl PgExecutor<'_>,
            client_id: &AsClientId,
        ) -> Result<(), StorageError> {
            sqlx::query!(
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
                r#"SELECT credential as "client_credential: FlatClientCredential" FROM as_client_records WHERE user_name = $1"#,
                user_name.to_string(),
            )
            .fetch_all(connection)
            .await?.into_iter()
                .map(|flat_credential| {
                    let client_credential = flat_credential.into();
                    Ok(client_credential)
                }).collect()
        }
    }
}
