// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use client_queue::ClientQueueData;
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{ratchet::QueueRatchet, RatchetEncryptionKey},
    identifiers::AsClientId,
    messages::{client_as::AsQueueMessagePayload, EncryptedAsQueueMessage},
    time::TimeStamp,
};
use sqlx::{Connection, PgConnection};

use crate::persistence::StorageError;

#[derive(Debug, Clone)]
pub(super) struct ClientRecord {
    pub(super) queue_encryption_key: RatchetEncryptionKey,
    pub(super) ratchet_key: QueueRatchet<EncryptedAsQueueMessage, AsQueueMessagePayload>,
    pub(super) activity_time: TimeStamp,
    pub(super) credential: ClientCredential,
}

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
        };

        // Initialize the client's queue.
        let mut transaction = connection.begin().await?;
        record.store(&mut transaction).await?;
        ClientQueueData::new_and_store(record.client_id(), &mut transaction).await?;
        transaction.commit().await?;

        Ok(record)
    }

    fn client_id(&self) -> AsClientId {
        self.credential.identity()
    }
}

mod client_queue {
    use phnxtypes::identifiers::AsClientId;
    use sqlx::PgConnection;

    use crate::persistence::StorageError;

    pub(super) struct ClientQueueData {
        queue_id: AsClientId,
        sequence_number: i64,
    }

    impl ClientQueueData {
        pub(super) async fn new_and_store<'a>(
            queue_id: AsClientId,
            connection: &mut PgConnection,
        ) -> Result<Self, StorageError> {
            let queue_data = Self {
                queue_id,
                sequence_number: 0,
            };
            queue_data.store(connection).await?;
            Ok(queue_data)
        }
    }

    mod persistence {
        use super::*;

        impl ClientQueueData {
            pub(super) async fn store(
                &self,
                connection: &mut PgConnection,
            ) -> Result<(), StorageError> {
                sqlx::query!(
                    "INSERT INTO as_queue_data (queue_id, sequence_number) VALUES ($1, $2)",
                    self.queue_id.client_id(),
                    self.sequence_number
                )
                .execute(connection)
                .await?;
                Ok(())
            }
        }
    }
}

mod persistence {
    use phnxtypes::codec::PhnxCodec;
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
                1000, // TODO: Once we use tokens, we should make this configurable.
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
                "UPDATE as_client_records SET queue_encryption_key = $1, ratchet = $2, activity_time = $3, client_credential = $4 WHERE client_id = $5",
                queue_encryption_key_bytes,
                ratchet,
                activity_time,
                client_credential,
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
                    client_credential
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
    }
}
