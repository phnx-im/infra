// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::Serialize;

use crate::{
    crypto::hpke::HpkeDecryptable,
    messages::{
        intra_backend::DsFanOutMessage,
        qs_qs::{QsToQsMessage, QsToQsPayload},
        MlsInfraVersion,
    },
};

use super::{
    errors::{QsEnqueueError, QsVerifyingKeyError},
    network_provider_trait::NetworkProvider,
    qs_api::FederatedProcessingResult,
    storage_provider_trait::QsStorageProvider,
    ClientConfig, Fqdn, Qs, QsVerifyingKey, WebsocketNotifier,
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
    pub async fn enqueue_message<S: QsStorageProvider, W: WebsocketNotifier, N: NetworkProvider>(
        storage_provider: &S,
        websocket_notifier: &W,
        network_provider: &N,
        message: DsFanOutMessage,
    ) -> Result<(), QsEnqueueError<S, N>> {
        let own_domain = storage_provider.own_domain().await;
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
            let decryption_key = storage_provider
                .load_decryption_key()
                .await
                .map_err(|_| QsEnqueueError::StorageError)?;
            let client_config = ClientConfig::decrypt(
                message.client_reference.sealed_reference,
                &decryption_key,
                &[],
                &[],
            )?;

            // Fetch the client record.
            let mut client_record = storage_provider
                .load_client(&client_config.client_id)
                .await
                .ok_or(QsEnqueueError::QueueNotFound)?;

            client_record
                .enqueue(
                    &client_config.client_id,
                    storage_provider,
                    websocket_notifier,
                    message.payload,
                    client_config.push_token_ear_key,
                )
                .await?
        }
        Ok(())

        // TODO: client now has new ratchet key, store it in the storage
        // provider.
    }

    /// Fetch the verifying key of the server with the given domain
    #[tracing::instrument(skip_all, err)]
    pub async fn verifying_key<S: QsStorageProvider, N: NetworkProvider>(
        storage_provider: &S,
        network_provider: &N,
        domain: Fqdn,
    ) -> Result<QsVerifyingKey, QsVerifyingKeyError> {
        let own_domain = storage_provider.own_domain().await;
        let verifying_key = if domain != own_domain {
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
            storage_provider
                .load_signing_key()
                .await
                .map_err(|_| QsVerifyingKeyError::StorageError)?
                .verifying_key()
                .clone()
        };
        Ok(verifying_key)
    }
}
