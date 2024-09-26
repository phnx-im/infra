// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    crypto::{hpke::HpkeDecryptable, signatures::keys::QsVerifyingKey},
    errors::qs::QsVerifyingKeyError,
    identifiers::{ClientConfig, Fqdn},
    messages::MlsInfraVersion,
};
use tls_codec::Serialize;

use crate::messages::{
    intra_backend::DsFanOutMessage,
    qs_qs::{QsToQsMessage, QsToQsPayload},
};

use super::{
    client_id_decryption_key::StorableClientIdDecryptionKey, client_record::QsClientRecord,
    errors::QsEnqueueError, network_provider_trait::NetworkProvider,
    qs_api::FederatedProcessingResult, signing_key::StorableQsSigningKey, PushNotificationProvider,
    Qs, WebsocketNotifier,
};

impl Qs {
    /// Enqueue the given message. This endpoint is called by the local DS
    /// during a fanout operation. This endpoint does not necessairly return
    /// quickly. It can attempt to do the full fanout and return potential
    /// failed transmissions to the DS.
    ///
    /// This endpoint is used for enqueining messages in both local and remote
    /// queues, depending on the FQDN of the client.
    #[tracing::instrument(skip_all, err)]
    pub async fn enqueue_message<
        W: WebsocketNotifier,
        N: NetworkProvider,
        P: PushNotificationProvider,
    >(
        &self,
        websocket_notifier: &W,
        push_notification_provider: &P,
        network_provider: &N,
        message: DsFanOutMessage,
    ) -> Result<(), QsEnqueueError<N>> {
        let own_domain = self.domain.clone();
        if message.client_reference.client_homeserver_domain != own_domain {
            let qs_to_qs_message = QsToQsMessage {
                protocol_version: MlsInfraVersion::Alpha,
                sender: own_domain.clone(),
                recipient: message.client_reference.client_homeserver_domain.clone(),
                payload: QsToQsPayload::FanOutMessageRequest(message.clone()),
            };
            let serialized_message = qs_to_qs_message
                .tls_serialize_detached()
                .map_err(|_| QsEnqueueError::LibraryError)?;
            network_provider
                .deliver(
                    serialized_message,
                    message.client_reference.client_homeserver_domain,
                )
                .await
                .map_err(QsEnqueueError::NetworkError)
                .and_then(|result| {
                    if matches!(result, FederatedProcessingResult::Ok) {
                        Ok(())
                    } else {
                        Err(QsEnqueueError::InvalidResponse)
                    }
                })?
        } else {
            let decryption_key = StorableClientIdDecryptionKey::load(&self.db_pool)
                .await
                .map_err(|_| QsEnqueueError::StorageError)?
                // There should always be a decryption key in the database.
                .ok_or(QsEnqueueError::LibraryError)?;
            let client_config = ClientConfig::decrypt(
                message.client_reference.sealed_reference,
                &decryption_key,
                &[],
                &[],
            )?;

            let mut transaction = self.db_pool.begin().await.map_err(|e| {
                tracing::warn!("Failed to start transaction: {:?}", e);
                QsEnqueueError::StorageError
            })?;

            // Fetch the client record.
            let mut client_record =
                QsClientRecord::load(&mut *transaction, &client_config.client_id)
                    .await
                    .map_err(|e| {
                        tracing::warn!("Failed to load client record: {:?}", e);
                        QsEnqueueError::StorageError
                    })?
                    .ok_or(QsEnqueueError::QueueNotFound)?;

            client_record
                .enqueue(
                    &mut transaction,
                    &client_config.client_id,
                    websocket_notifier,
                    push_notification_provider,
                    message.payload,
                    client_config.push_token_ear_key,
                )
                .await?;

            transaction.commit().await.map_err(|e| {
                tracing::warn!("Failed to commit transaction: {:?}", e);
                QsEnqueueError::StorageError
            })?;
        }
        Ok(())
    }

    /// Fetch the verifying key of the server with the given domain
    #[tracing::instrument(skip_all, err)]
    pub async fn verifying_key<N: NetworkProvider>(
        &self,
        network_provider: &N,
        domain: Fqdn,
    ) -> Result<QsVerifyingKey, QsVerifyingKeyError> {
        let own_domain = &self.domain;
        let verifying_key = if &domain != own_domain {
            let qs_to_qs_message = QsToQsMessage {
                protocol_version: MlsInfraVersion::Alpha,
                sender: own_domain.clone(),
                recipient: domain.clone(),
                payload: QsToQsPayload::VerificationKeyRequest,
            };
            let serialized_message = qs_to_qs_message
                .tls_serialize_detached()
                .map_err(|_| QsVerifyingKeyError::LibraryError)?;
            let result = network_provider
                .deliver(serialized_message, domain)
                .await
                .map_err(|_| QsVerifyingKeyError::InvalidResponse)?;
            if let FederatedProcessingResult::VerifyingKey(verifying_key) = result {
                verifying_key
            } else {
                return Err(QsVerifyingKeyError::InvalidResponse);
            }
        } else {
            StorableQsSigningKey::load(&self.db_pool)
                .await
                .map_err(|e| {
                    tracing::warn!("Failed to load signing key: {:?}", e);
                    QsVerifyingKeyError::StorageError
                })?
                .ok_or(QsVerifyingKeyError::LibraryError)?
                .verifying_key()
                .clone()
        };
        Ok(verifying_key)
    }
}
