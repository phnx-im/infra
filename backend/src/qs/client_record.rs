// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use phnxtypes::{
    crypto::{
        RatchetEncryptionKey, RatchetKeyUpdate,
        ear::{EarDecryptable, keys::PushTokenEarKey},
        signatures::keys::QsClientVerifyingKey,
    },
    identifiers::{QsClientId, QsUserId},
    messages::{
        QueueMessage,
        client_ds::QsQueueRatchet,
        push_token::{EncryptedPushToken, PushToken},
    },
    time::TimeStamp,
};

use crate::{
    errors::StorageError,
    messages::intra_backend::DsFanOutPayload,
    qs::{Notification, PushNotificationError},
};

use super::{Notifier, PushNotificationProvider, errors::EnqueueError, queue::Queue};

/// An enum defining the different kind of messages that are stored in an QS
/// queue.
/// TODO: This needs a codec that allows decoding to the proper type.
#[derive(Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize, Debug)]
#[repr(u8)]
pub(super) enum QueueMessageType {
    #[tls_codec(discriminant = 1)]
    RatchetKeyUpdate(RatchetKeyUpdate),
    EnqueuedMessage(QueueMessage),
}

/// Info attached to a queue meant as a target for messages fanned out by a DS.
#[derive(Clone, Debug, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub(super) struct QsClientRecord {
    pub(super) user_id: QsUserId,
    pub(super) client_id: QsClientId,
    pub(super) encrypted_push_token: Option<EncryptedPushToken>,
    pub(super) queue_encryption_key: RatchetEncryptionKey,
    pub(super) auth_key: QsClientVerifyingKey,
    pub(super) ratchet_key: QsQueueRatchet,
    pub(super) activity_time: TimeStamp,
}

impl QsClientRecord {
    #[expect(clippy::too_many_arguments)]
    pub(super) async fn new_and_store(
        connection: &mut PgConnection,
        rng: &mut (impl CryptoRng + RngCore),
        now: TimeStamp,
        user_id: QsUserId,
        encrypted_push_token: Option<EncryptedPushToken>,
        queue_encryption_key: RatchetEncryptionKey,
        auth_key: QsClientVerifyingKey,
        ratchet_key: QsQueueRatchet,
    ) -> Result<Self, StorageError> {
        let client_id = QsClientId::random(rng);

        let mut transaction = connection.begin().await?;

        let record = Self {
            user_id,
            client_id,
            encrypted_push_token,
            queue_encryption_key,
            auth_key,
            ratchet_key,
            activity_time: now,
        };
        record.store(&mut *transaction).await?;

        Queue::new_and_store(client_id, &mut *transaction).await?;

        transaction.commit().await?;

        Ok(record)
    }
}

pub(crate) mod persistence {
    use phnxtypes::codec::{BlobDecoded, BlobEncoded};
    use sqlx::{PgExecutor, query};

    use super::*;

    use crate::errors::StorageError;

    impl QsClientRecord {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            // Create and store the client record.
            let owner_public_key = BlobEncoded(&self.queue_encryption_key);
            let owner_signature_key = BlobEncoded(&self.auth_key);
            let ratchet = BlobEncoded(&self.ratchet_key);

            query!(
                "INSERT INTO
                    qs_client_records
                    (client_id, user_id, encrypted_push_token, owner_public_key,
                    owner_signature_key, ratchet, activity_time)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7)",
                &self.client_id as &QsClientId,
                &self.user_id as &QsUserId,
                self.encrypted_push_token.as_ref() as Option<&EncryptedPushToken>,
                owner_public_key as _,
                owner_signature_key as _,
                ratchet as _,
                &self.activity_time as _,
            )
            .execute(connection)
            .await?;

            Ok(())
        }

        pub(in crate::qs) async fn load(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
        ) -> Result<Option<QsClientRecord>, StorageError> {
            let client_id = client_id.as_uuid();
            let record = sqlx::query!(
                r#"SELECT
                    user_id as "user_id: QsUserId",
                    encrypted_push_token as "encrypted_push_token: EncryptedPushToken",
                    owner_public_key AS "owner_public_key: BlobDecoded<RatchetEncryptionKey>",
                    owner_signature_key AS "owner_signature_key: BlobDecoded<QsClientVerifyingKey>",
                    ratchet AS "ratchet: BlobDecoded<QsQueueRatchet>",
                    activity_time AS "activity_time: TimeStamp"
                FROM
                    qs_client_records
                WHERE
                    client_id = $1"#,
                client_id,
            )
            .fetch_optional(connection)
            .await?;
            Ok(record.map(|record| QsClientRecord {
                user_id: record.user_id,
                client_id: (*client_id).into(),
                encrypted_push_token: record.encrypted_push_token,
                queue_encryption_key: record.owner_public_key.into_inner(),
                auth_key: record.owner_signature_key.into_inner(),
                ratchet_key: record.ratchet.into_inner(),
                activity_time: record.activity_time,
            }))
        }

        pub(in crate::qs) async fn update(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            let owner_public_key = BlobEncoded(&self.queue_encryption_key);
            let owner_signature_key = BlobEncoded(&self.auth_key);
            let ratchet = BlobEncoded(&self.ratchet_key);

            query!(
                "UPDATE qs_client_records
                SET
                    encrypted_push_token = $1,
                    owner_public_key = $2,
                    owner_signature_key = $3,
                    ratchet = $4,
                    activity_time = $5
                WHERE
                    client_id = $6",
                self.encrypted_push_token.as_ref() as Option<&EncryptedPushToken>,
                owner_public_key as _,
                owner_signature_key as _,
                ratchet as _,
                &self.activity_time as &TimeStamp,
                &self.client_id as &QsClientId,
            )
            .execute(connection)
            .await?;

            Ok(())
        }

        pub(in crate::qs) async fn delete(
            connection: impl PgExecutor<'_>,
            client_id: &QsClientId,
        ) -> Result<(), StorageError> {
            query!(
                "DELETE FROM qs_client_records WHERE client_id = $1",
                client_id as &QsClientId
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) mod tests {
        use phnxtypes::crypto::ratchet::QueueRatchet;
        use sqlx::PgPool;

        use crate::qs::user_record::persistence::tests::store_random_user_record;

        use super::*;

        fn random_client_record(user_id: QsUserId) -> QsClientRecord {
            QsClientRecord {
                user_id,
                client_id: QsClientId::random(&mut rand::thread_rng()),
                encrypted_push_token: Some(EncryptedPushToken::dummy()),
                queue_encryption_key: RatchetEncryptionKey::new_for_test(
                    b"encryption_key_32_bytes".to_vec(),
                ),
                auth_key: QsClientVerifyingKey::new_for_test(b"auth_key".to_vec()),
                ratchet_key: QueueRatchet::random().unwrap(),
                activity_time: TimeStamp::now(),
            }
        }

        pub(crate) async fn store_random_client_record(
            pool: &PgPool,
            user_id: QsUserId,
        ) -> anyhow::Result<QsClientRecord> {
            let record = random_client_record(user_id);
            record.store(pool).await?;
            Ok(record)
        }

        #[sqlx::test]
        async fn store(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(&pool, user_record.user_id).await?;

            let loaded = QsClientRecord::load(&pool, &client_record.client_id)
                .await?
                .expect("missing client record");
            assert_eq!(loaded, client_record);

            Ok(())
        }

        #[sqlx::test]
        async fn update(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(&pool, user_record.user_id).await?;

            let loaded = QsClientRecord::load(&pool, &client_record.client_id)
                .await?
                .expect("missing client record");
            assert_eq!(loaded, client_record);

            let updated_client_record = QsClientRecord {
                user_id: client_record.user_id,
                client_id: client_record.client_id,
                ..random_client_record(user_record.user_id)
            };

            updated_client_record.update(&pool).await?;
            let loaded = QsClientRecord::load(&pool, &client_record.client_id)
                .await?
                .expect("missing client record");
            assert_eq!(loaded, updated_client_record);

            Ok(())
        }

        #[sqlx::test]
        async fn delete(pool: PgPool) -> anyhow::Result<()> {
            let user_record = store_random_user_record(&pool).await?;
            let client_record = store_random_client_record(&pool, user_record.user_id).await?;

            let loaded = QsClientRecord::load(&pool, &client_record.client_id)
                .await?
                .expect("missing client record");
            assert_eq!(loaded, client_record);

            QsClientRecord::delete(&pool, &client_record.client_id).await?;
            let loaded = QsClientRecord::load(&pool, &client_record.client_id).await?;
            assert_eq!(loaded, None);

            Ok(())
        }
    }
}

impl QsClientRecord {
    /// Put a message into the queue.
    pub(crate) async fn enqueue<W: Notifier, P: PushNotificationProvider>(
        &mut self,
        connection: &mut PgConnection,
        client_id: &QsClientId,
        websocket_notifier: &W,
        push_notification_provider: &P,
        msg: DsFanOutPayload,
        push_token_key_option: Option<PushTokenEarKey>,
    ) -> Result<(), EnqueueError> {
        match msg {
            // Enqueue a queue message.
            // Serialize the message so that we can put it in the queue.
            DsFanOutPayload::QueueMessage(queue_message) => {
                // Encrypt the message under the current ratchet key.
                let queue_message = self
                    .ratchet_key
                    .encrypt(queue_message)
                    .map_err(|_| EnqueueError::LibraryError)?;

                // TODO: Future work: PCS

                tracing::trace!("Enqueueing message in storage provider");
                Queue::enqueue(connection, &self.client_id, &queue_message)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to enqueue message: {:?}", e);
                        EnqueueError::Storage
                    })?;

                // Try to send a notification over the websocket, otherwise use push tokens if available
                if websocket_notifier
                    .notify(client_id, Notification::QueueUpdate)
                    .await
                    .is_err()
                {
                    // Send a push notification under the following conditions:
                    // - there is a push token associated with the queue
                    // - there is a push token decryption key
                    // - the decryption is successful
                    if let Some(ref encrypted_push_token) = self.encrypted_push_token {
                        if let Some(ref ear_key) = push_token_key_option {
                            // Attempt to decrypt the push token.
                            match PushToken::decrypt(ear_key, encrypted_push_token) {
                                Err(e) => {
                                    tracing::error!("Push token decryption failed: {}", e);
                                }
                                Ok(push_token) => {
                                    // Send the push notification.
                                    if let Err(e) =
                                        push_notification_provider.push(push_token).await
                                    {
                                        match e {
                                            // The push notification failed for some other reason.
                                            PushNotificationError::Other(error_description) => {
                                                tracing::error!(
                                                    "Push notification failed unexpectedly: {}",
                                                    error_description
                                                )
                                            }
                                            // The token is no longer valid and should be deleted.
                                            PushNotificationError::InvalidToken(
                                                error_description,
                                            ) => {
                                                tracing::info!(
                                                    "Push notification failed because the token is invalid: {}",
                                                    error_description
                                                );
                                                self.encrypted_push_token = None;
                                            }
                                            // There was a network error when trying to send the push notification.
                                            PushNotificationError::NetworkError(e) => {
                                                tracing::info!(
                                                    "Push notification failed because of a network error: {}",
                                                    e
                                                )
                                            }
                                            PushNotificationError::UnsupportedType => {
                                                tracing::warn!(
                                                    "Push notification failed because the push token type is unsupported",
                                                )
                                            }
                                            PushNotificationError::JwtCreationError(e) => {
                                                tracing::error!(
                                                    "Push notification failed because the JWT token could not be created: {}",
                                                    e
                                                )
                                            }
                                            PushNotificationError::OAuthError(e) => {
                                                tracing::error!(
                                                    "Push notification failed because of an OAuth error: {}",
                                                    e
                                                )
                                            }
                                            PushNotificationError::InvalidConfiguration(e) => {
                                                tracing::error!(
                                                    "Push notification failed because of an invalid configuration: {}",
                                                    e
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // We also update th client record in the storage provider,
                // since we need to store the new ratchet key and because we
                // might have deleted the push token.
                self.update(connection).await.map_err(|e| {
                    tracing::error!("Failed to update client record: {:?}", e);
                    EnqueueError::Storage
                })?;
            }
            // Dispatch an event message.
            DsFanOutPayload::EventMessage(event_message) => {
                // We ignore the result, because dispatching events is best effort.œ
                let _ = websocket_notifier
                    .notify(client_id, Notification::Event(event_message))
                    .await;
            }
        }

        // Success!
        Ok(())
    }
}
