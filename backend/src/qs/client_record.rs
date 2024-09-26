// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, PgConnection};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use phnxtypes::{
    crypto::{
        ear::{keys::PushTokenEarKey, EarDecryptable},
        ratchet::QueueRatchet,
        signatures::keys::QsClientVerifyingKey,
        RatchetEncryptionKey, RatchetKeyUpdate,
    },
    identifiers::{QsClientId, QsUserId},
    messages::{
        client_ds::QsQueueMessagePayload,
        push_token::{EncryptedPushToken, PushToken},
        EncryptedQsQueueMessage, QueueMessage,
    },
    time::TimeStamp,
};

use crate::{
    messages::intra_backend::DsFanOutPayload,
    persistence::StorageError,
    qs::{PushNotificationError, WsNotification},
};

use super::{errors::EnqueueError, queue::Queue, PushNotificationProvider, WebsocketNotifier};

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
#[derive(
    Clone, Debug, PartialEq, Serialize, Deserialize, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub(super) struct QsClientRecord {
    pub(super) user_id: QsUserId,
    pub(super) client_id: QsClientId,
    pub(super) encrypted_push_token: Option<EncryptedPushToken>,
    pub(super) queue_encryption_key: RatchetEncryptionKey,
    pub(super) auth_key: QsClientVerifyingKey,
    pub(super) ratchet_key: QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>,
    pub(super) activity_time: TimeStamp,
}

impl QsClientRecord {
    pub(super) async fn new_and_store(
        connection: &mut PgConnection,
        rng: &mut (impl CryptoRng + RngCore),
        now: TimeStamp,
        user_id: QsUserId,
        encrypted_push_token: Option<EncryptedPushToken>,
        queue_encryption_key: RatchetEncryptionKey,
        auth_key: QsClientVerifyingKey,
        ratchet_key: QueueRatchet<EncryptedQsQueueMessage, QsQueueMessagePayload>,
    ) -> Result<Self, StorageError> {
        let client_id = QsClientId::random(rng);

        let mut transaction = connection.begin().await?;

        let record = Self {
            user_id,
            client_id: client_id.clone(),
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

mod persistence {
    use phnxtypes::codec::PhnxCodec;
    use sqlx::{PgConnection, PgExecutor};

    use super::*;

    use crate::persistence::StorageError;

    impl QsClientRecord {
        pub(super) async fn store(
            &self,
            connection: impl PgExecutor<'_>,
        ) -> Result<(), StorageError> {
            // Create and store the client record.
            let owner_public_key = PhnxCodec::to_vec(&self.queue_encryption_key)?;
            let owner_signature_key = PhnxCodec::to_vec(&self.auth_key)?;
            let ratchet = PhnxCodec::to_vec(&self.ratchet_key)?;

            sqlx::query!(
                "INSERT INTO 
                    qs_client_records 
                    (client_id, user_id, encrypted_push_token, owner_public_key, owner_signature_key, ratchet, activity_time) 
                VALUES 
                    ($1, $2, $3, $4, $5, $6, $7)", 
                &self.client_id as &QsClientId,
                &self.user_id as &QsUserId,
                self.encrypted_push_token.as_ref() as Option<&EncryptedPushToken>,
                owner_public_key,
                owner_signature_key,
                ratchet,
                &self.activity_time as &TimeStamp,
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
            sqlx::query!(
                r#"SELECT 
                    user_id as "user_id: QsUserId", 
                    encrypted_push_token as "encrypted_push_token: EncryptedPushToken", 
                    owner_public_key, 
                    owner_signature_key, 
                    ratchet, 
                    activity_time as "activity_time: TimeStamp"
                FROM 
                    qs_client_records 
                WHERE 
                    client_id = $1"#,
                client_id,
            )
            .fetch_optional(connection)
            .await?
            .map(|record| {
                let owner_public_key = PhnxCodec::from_slice(&record.owner_public_key)?;
                let owner_signature_key = PhnxCodec::from_slice(&record.owner_signature_key)?;
                let ratchet = PhnxCodec::from_slice(&record.ratchet)?;
                let ratchet_key = QueueRatchet::from(ratchet);

                Ok(QsClientRecord {
                    user_id: record.user_id,
                    client_id: client_id.clone().into(),
                    encrypted_push_token: record.encrypted_push_token,
                    queue_encryption_key: owner_public_key,
                    auth_key: owner_signature_key,
                    ratchet_key,
                    activity_time: record.activity_time,
                })
            })
            .transpose()
        }

        pub(in crate::qs) async fn update(
            &self,
            connection: &mut PgConnection,
        ) -> Result<(), StorageError> {
            let owner_public_key = PhnxCodec::to_vec(&self.queue_encryption_key)?;
            let owner_signature_key = PhnxCodec::to_vec(&self.auth_key)?;
            let ratchet = PhnxCodec::to_vec(&self.ratchet_key)?;

            sqlx::query!(
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
                owner_public_key,
                owner_signature_key,
                ratchet,
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
            sqlx::query!(
                "DELETE FROM qs_client_records WHERE client_id = $1",
                client_id as &QsClientId
            )
            .execute(connection)
            .await?;
            Ok(())
        }
    }
}

impl QsClientRecord {
    /// Put a message into the queue.
    pub(crate) async fn enqueue<W: WebsocketNotifier, P: PushNotificationProvider>(
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
                Queue::enqueue(connection, &self.client_id, queue_message)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to enqueue message: {:?}", e);
                        EnqueueError::Storage
                    })?;

                // Try to send a notification over the websocket, otherwise use push tokens if available
                if websocket_notifier
                    .notify(client_id, WsNotification::QueueUpdate)
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
                                    PushNotificationError::InvalidToken(error_description) => {
                                        tracing::info!(
                                            "Push notification failed because the token is invalid: {}",
                                            error_description
                                        );
                                        self.encrypted_push_token = None;
                                    }
                                    // There was a network error when trying to send the push notification.
                                    PushNotificationError::NetworkError(e) => tracing::info!(
                                        "Push notification failed because of a network error: {}",
                                        e
                                    ),
                                    PushNotificationError::UnsupportedType => tracing::warn!(
                                        "Push notification failed because the push token type is unsupported",
                                    ),
                                    PushNotificationError::JwtCreationError(e) => tracing::error!(
                                        "Push notification failed because the JWT token could not be created: {}",
                                        e
                                    ),
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
                    .notify(client_id, WsNotification::Event(event_message))
                    .await;
            }
        }

        // Success!
        Ok(())
    }
}
