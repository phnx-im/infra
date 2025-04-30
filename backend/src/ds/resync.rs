// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage, messages::SerializedMlsMessage, openmls::prelude::Sender,
    provider_traits::MlsAssistProvider,
};
use mls_assist::{
    messages::AssistedMessageIn,
    openmls::prelude::{LeafNodeIndex, ProcessedMessageContent},
};
use phnxtypes::{errors::ResyncClientError, time::Duration};

use super::process::USER_EXPIRATION_DAYS;

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn resync_client(
        &mut self,
        external_commit: AssistedMessageIn,
        sender_index: LeafNodeIndex,
    ) -> Result<SerializedMlsMessage, ResyncClientError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(self.provider.crypto(), external_commit)
            .map_err(|_| ResyncClientError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(processed_message, _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                return Err(ResyncClientError::InvalidMessage);
            };

        let ProcessedMessageContent::StagedCommitMessage(staged_commit_message) =
            processed_message.content()
        else {
            // This should be a staged commit message.
            return Err(ResyncClientError::InvalidMessage);
        };

        // Check if it's an external commit.
        if !matches!(processed_message.sender(), Sender::NewMemberCommit) {
            return Err(ResyncClientError::InvalidMessage);
        }

        let Some(remove_proposal) = staged_commit_message.remove_proposals().next() else {
            // This should contain a remove proposal.
            return Err(ResyncClientError::InvalidMessage);
        };

        if remove_proposal.remove_proposal().removed() != sender_index {
            // The sender index in the remove proposal should match the sender
            // index in the params.
            return Err(ResyncClientError::InvalidMessage);
        }

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We just accept the message into the group state.
        self.group.accept_processed_message(
            self.provider.storage(),
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        // No need to update the user profile, since the client was re-added on
        // the same position.
        // No need to update the client profile either, since all data (leaf
        // index, credential, qs client ref, etc.) remains the same.

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
