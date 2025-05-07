// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::qs::QsDequeueError,
    messages::client_qs::{DequeueMessagesParams, DequeueMessagesResponse},
};

use super::{Qs, queue::Queue};

pub(crate) mod client_records;
pub(crate) mod key_packages;
pub(crate) mod user_records;

impl Qs {
    /// Retrieve messages the given number of messages, starting with
    /// `sequence_number_start` from the queue with the given id and delete any
    /// messages older than the given sequence number start.
    #[tracing::instrument(skip_all, err)]
    pub(crate) async fn qs_dequeue_messages(
        &self,
        params: DequeueMessagesParams,
    ) -> Result<DequeueMessagesResponse, QsDequeueError> {
        let DequeueMessagesParams {
            sender,
            sequence_number_start,
            max_message_number,
        } = params;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::warn!("Failed to acquire connection: {:?}", e);
            QsDequeueError::StorageError
        })?;
        let (messages, remaining_messages_number) = Queue::read_and_delete(
            &mut connection,
            &sender,
            sequence_number_start,
            max_message_number,
        )
        .await
        .map_err(|e| {
            tracing::warn!("Storage provider error: {:?}", e);
            QsDequeueError::StorageError
        })?;

        let response = DequeueMessagesResponse {
            messages,
            remaining_messages_number,
        };

        Ok(response)
    }
}
