// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircommon::time::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::{AssistedMessageIn, SerializedMlsMessage},
    openmls::prelude::ProcessedMessageContent,
    provider_traits::MlsAssistProvider,
};

use crate::errors::ClientUpdateError;

use super::{group_state::DsGroupState, process::USER_EXPIRATION_DAYS};

impl DsGroupState {
    pub(super) fn update_client(
        &mut self,
        commit: AssistedMessageIn,
    ) -> Result<SerializedMlsMessage, ClientUpdateError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(self.provider.crypto(), commit)
            .map_err(|_| ClientUpdateError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(processed_message, _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                tracing::warn!("Invalid message type");
                return Err(ClientUpdateError::InvalidMessage);
            };

        if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
            {
                tracing::warn!("Client update message contained add or update proposals");
                return Err(ClientUpdateError::InvalidMessage);
            }
        } else {
            tracing::warn!("Client update message was not a commit");
            return Err(ClientUpdateError::InvalidMessage);
        };

        // Finalize processing.
        self.group.accept_processed_message(
            self.provider.storage(),
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
