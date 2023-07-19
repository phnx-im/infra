// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::Serialize;

use crate::{crypto::hpke::HpkeDecryptable, messages::intra_backend::DsFanOutMessage};

use super::{
    errors::QsEnqueueError, network_provider_trait::NetworkProvider,
    storage_provider_trait::QsStorageProvider, ClientConfig, Qs, WebsocketNotifier,
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
        if message.client_reference.client_homeserver_domain != storage_provider.own_domain().await
        {
            tracing::info!(
                "Domains differ. Destination domain: {:?}, own domain: {:?}",
                message.client_reference.client_homeserver_domain,
                storage_provider.own_domain().await
            );
            let serialized_message = message
                .tls_serialize_detached()
                .map_err(|_| QsEnqueueError::LibraryError)?;
            network_provider
                .deliver(
                    serialized_message,
                    message.client_reference.client_homeserver_domain,
                )
                .await
                .map_err(QsEnqueueError::NetworkError)?;
        }
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

        Ok(client_record
            .enqueue(
                &client_config.client_id,
                storage_provider,
                websocket_notifier,
                message.payload,
                client_config.push_token_ear_key,
            )
            .await?)

        // TODO: client now has new ratchet key, store it in the storage
        // provider.
    }
}
