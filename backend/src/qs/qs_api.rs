// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use crate::messages::qs_qs::QsToQsMessage;

use super::{Qs, QsConnector};

#[derive(Error, Debug, Clone)]
pub enum FederatedEnqueueError<C: QsConnector> {
    /// Error enqueueing message
    #[error("Error enqueueing message")]
    EnqueueError(C::EnqueueError),
}

impl Qs {
    /// Enqueue the given message. This endpoint is called by a remote QS
    /// during a fanout operation. This endpoint does not necessairly return
    /// quickly. It can attempt to do the full fanout and return potential
    /// failed transmissions to the remote QS.
    #[tracing::instrument(skip_all, err)]
    pub async fn enqueue_remote_message<C: QsConnector>(
        qs_connector: &C,
        message: QsToQsMessage,
    ) -> Result<(), FederatedEnqueueError<C>> {
        let QsToQsMessage {
            protocol_version: _,
            sender: _,
            recipient: _,
            fan_out_message,
        } = message;
        // TODO: validation
        qs_connector
            .dispatch(fan_out_message)
            .await
            .map_err(FederatedEnqueueError::EnqueueError)?;
        Ok(())
    }
}
