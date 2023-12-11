// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::errors::qs::QsVerifyingKeyError;
use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::messages::qs_qs::{QsToQsMessage, QsToQsPayload};

use super::{
    errors::QsEnqueueError, network_provider_trait::NetworkProvider,
    storage_provider_trait::QsStorageProvider, Qs, QsVerifyingKey, WebsocketNotifier,
};

#[derive(Error, Debug)]
pub enum FederatedProcessingError<S: QsStorageProvider, N: NetworkProvider> {
    /// Error enqueueing message
    #[error(transparent)]
    EnqueueError(#[from] QsEnqueueError<S, N>),
    /// Error getting verifying key
    #[error(transparent)]
    VerifyingKeyError(#[from] QsVerifyingKeyError),
}

#[derive(Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum FederatedProcessingResult {
    Ok,
    VerifyingKey(QsVerifyingKey),
}

impl Qs {
    /// Process the QsToQsMessage.
    pub async fn process_federated_message<
        S: QsStorageProvider,
        W: WebsocketNotifier,
        N: NetworkProvider,
    >(
        storage_provider: &S,
        websocket_notifier: &W,
        network_provider: &N,
        message: QsToQsMessage,
    ) -> Result<FederatedProcessingResult, FederatedProcessingError<S, N>> {
        let QsToQsMessage {
            protocol_version: _,
            sender: _,
            recipient: _,
            payload,
        } = message;
        // TODO: validation. Also: Signatures. In particular, we need to check
        // that the fqdn in the client references is actually ours otherwise,
        // other QSs can route messages through us.
        let result = match payload {
            QsToQsPayload::FanOutMessageRequest(fan_out_message) => {
                Self::enqueue_message(
                    storage_provider,
                    websocket_notifier,
                    network_provider,
                    fan_out_message,
                )
                .await?;
                FederatedProcessingResult::Ok
            }
            QsToQsPayload::VerificationKeyRequest => {
                let verifying_key_response = Self::qs_verifying_key(storage_provider).await?;
                FederatedProcessingResult::VerifyingKey(verifying_key_response.verifying_key)
            }
        };
        Ok(result)
    }
}
