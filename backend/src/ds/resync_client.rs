// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::SerializedMlsMessage,
    openmls::prelude::{ProcessedMessageContent, Sender},
};
use phnxtypes::{
    errors::ResyncClientError, messages::client_ds::ResyncClientParams, time::Duration,
};

use super::api::USER_EXPIRATION_DAYS;

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn resync_client(
        &mut self,
        params: ResyncClientParams,
    ) -> Result<SerializedMlsMessage, ResyncClientError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.external_commit)
            .map_err(|_| ResyncClientError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                return Err(ResyncClientError::InvalidMessage);
            };

        let removed_client = if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            // Check that the commit only contains removes.
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
            {
                return Err(ResyncClientError::InvalidMessage);
            }

            if let Some(leaf_index) = staged_commit
                .remove_proposals()
                .map(|remove_proposal| remove_proposal.remove_proposal().removed())
                .next()
            {
                leaf_index
            } else {
                return Err(ResyncClientError::InvalidMessage);
            }
        } else {
            return Err(ResyncClientError::InvalidMessage);
        };

        // Check if it's an external commit.
        if let Sender::NewMemberCommit = processed_message.sender() {
            return Err(ResyncClientError::InvalidMessage);
        }

        // Check if the removed client belongs to the sending user.
        if !self
            .user_profiles
            .get(&params.sender)
            // There should be a user profile. If there wasn't, verification should have failed.
            .ok_or(ResyncClientError::LibraryError)?
            .clients
            .contains(&removed_client)
        {
            return Err(ResyncClientError::InvalidMessage);
        };

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We just accept the message into the group state.
        self.group_mut().accept_processed_message(
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // No need to update the user profile, since the client was re-added on
        // the same position.
        // No need to update the client profile either, since all data (leaf
        // index, credential, qs client ref, etc.) remains the same.

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
