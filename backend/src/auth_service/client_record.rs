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

use crate::persistence::StorageError;

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
    use phnxtypes::{codec::PhnxCodec, identifiers::QualifiedUserName};
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
            let queue_encryption_key_bytes = PhnxCodec::to_vec(&self.queue_encryption_key)?;
            let ratchet = PhnxCodec::to_vec(&self.ratchet_key)?;
            let activity_time = DateTime::<Utc>::from(self.activity_time);
            let client_credential = PhnxCodec::to_vec(&self.credential)?;
            let client_id = self.credential.identity();
            sqlx::query!(
                "INSERT INTO as_client_records (client_id, user_name, queue_encryption_key, ratchet, activity_time, client_credential, remaining_tokens) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                client_id.client_id(),
                client_id.user_name().to_string(),
                queue_encryption_key_bytes,
                ratchet,
                activity_time,
                client_credential,
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
            let queue_encryption_key_bytes = PhnxCodec::to_vec(&self.queue_encryption_key)?;
            let ratchet = PhnxCodec::to_vec(&self.ratchet_key)?;
            let activity_time = DateTime::<Utc>::from(self.activity_time);
            let client_credential = PhnxCodec::to_vec(&self.credential)?;
            let client_id = self.credential.identity();
            sqlx::query!(
                "UPDATE as_client_records SET queue_encryption_key = $1, ratchet = $2, activity_time = $3, client_credential = $4, remaining_tokens = $5 WHERE client_id = $6",
                queue_encryption_key_bytes,
                ratchet,
                activity_time,
                client_credential,
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
            sqlx::query!(
                "SELECT
                    queue_encryption_key,
                    ratchet,
                    activity_time,
                    client_credential,
                    remaining_tokens
                FROM as_client_records WHERE client_id = $1",
                client_id.client_id(),
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                let queue_encryption_key = PhnxCodec::from_slice(&record.queue_encryption_key)?;
                let ratchet = PhnxCodec::from_slice(&record.ratchet)?;
                let activity_time = record.activity_time.into();
                let client_credential = PhnxCodec::from_slice(&record.client_credential)?;
                Ok(ClientRecord {
                    queue_encryption_key,
                    ratchet_key: ratchet,
                    activity_time,
                    credential: client_credential,
                    token_allowance: record.remaining_tokens,
                })
            })
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
            let client_records = sqlx::query!(
                "SELECT client_credential FROM as_client_records WHERE user_name = $1",
                user_name.to_string(),
            )
            .fetch_all(connection)
            .await?;
            client_records
                .into_iter()
                .map(|record| {
                    let client_credential = PhnxCodec::from_slice(&record.client_credential)?;
                    Ok(client_credential)
                })
                .collect()
        }
    }
}
