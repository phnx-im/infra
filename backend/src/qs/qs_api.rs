// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::messages::qs_qs::{QsToQsMessage, QsToQsPayload};

use super::{errors::QsEnqueueError, network_provider_trait::NetworkProvider, Qs, QsConnector};

#[derive(Error, Debug)]
pub enum FederatedProcessingError<N: NetworkProvider> {
    /// Error enqueueing message
    #[error(transparent)]
    EnqueueError(#[from] QsEnqueueError<N>),
}

#[derive(Debug, Clone, TlsSerialize, TlsSize, TlsDeserializeBytes)]
#[repr(u8)]
pub enum FederatedProcessingResult {
    Ok,
}

impl Qs {
    /// Process the QsToQsMessage.
    pub async fn process_federated_message<
        Qc: QsConnector<EnqueueError = QsEnqueueError<N>>,
        N: NetworkProvider,
    >(
        &self,
        qs_connector: &Qc,
        message: QsToQsMessage,
    ) -> Result<FederatedProcessingResult, FederatedProcessingError<N>> {
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
                qs_connector
                    .dispatch(fan_out_message)
                    .await
                    .map_err(FederatedProcessingError::EnqueueError)?;
                FederatedProcessingResult::Ok
            }
        };
        Ok(result)
    }
}
