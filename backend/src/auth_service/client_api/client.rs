// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxtypes::{
    errors::auth_service::AsDequeueError,
    messages::{client_as::DequeueMessagesParamsTbs, client_qs::DequeueMessagesResponse},
};

use crate::auth_service::{AuthService, queue::Queue};

impl AuthService {
    pub(crate) async fn as_dequeue_messages(
        &self,
        params: DequeueMessagesParamsTbs,
    ) -> Result<DequeueMessagesResponse, AsDequeueError> {
        let DequeueMessagesParamsTbs {
            sender,
            sequence_number_start,
            max_message_number,
        } = params;

        // TODO: The backend should have its own value for max_messages and use
        // that one if the client-given one exceeds it.
        tracing::trace!("Reading and deleting messages from storage provider");
        let mut connection = self.db_pool.acquire().await.map_err(|e| {
            tracing::error!("Error acquiring connection: {:?}", e);
            AsDequeueError::StorageError
        })?;
        let (messages, remaining_messages_number) = Queue::read_and_delete(
            &mut connection,
            &sender,
            sequence_number_start,
            max_message_number,
        )
        .await
        .map_err(|e| {
            tracing::error!("Storage provider error: {:?}", e);
            AsDequeueError::StorageError
        })?;

        let response = DequeueMessagesResponse {
            messages,
            remaining_messages_number,
        };

        Ok(response)
    }
}
