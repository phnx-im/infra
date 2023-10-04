// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage,
    openmls::prelude::{ProcessedMessageContent, Sender},
};
use phnxtypes::messages::client_ds::DeleteGroupParams;

use crate::messages::intra_backend::DsFanOutPayload;

use super::errors::GroupDeletionError;

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn delete_group(
        &mut self,
        params: DeleteGroupParams,
    ) -> Result<DsFanOutPayload, GroupDeletionError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.commit)
            .map_err(|_| GroupDeletionError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                tracing::warn!("Received non-commit message for delete_group operation");
                return Err(GroupDeletionError::InvalidMessage);
            };

        // Check if sender index and user profile match.
        let sender_index = if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(GroupDeletionError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                tracing::warn!("Missing user profile");
                return Err(GroupDeletionError::InvalidMessage);
            };
            leaf_index
        } else {
            // Remove users should be a regular commit
            tracing::warn!("Invalid sender");
            return Err(GroupDeletionError::InvalidMessage);
        };

        if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            // Check that the commit only contains removes.
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
            {
                tracing::warn!("Found add or update proposals in delete group commit");
                return Err(GroupDeletionError::InvalidMessage);
            }
            // Process remove proposals, but only non-inline ones.
            let removed_clients: Vec<_> = staged_commit
                .remove_proposals()
                .map(|remove_proposal| remove_proposal.remove_proposal().removed())
                .collect();
            let existing_clients: Vec<_> = self
                .client_profiles
                .keys()
                .filter(|index| index != &sender_index)
                .copied()
                .collect();
            // Check that we're indeed removing all the clients.
            if removed_clients != existing_clients {
                tracing::warn!("Incomplete remove proposals in delete group commit");
                return Err(GroupDeletionError::InvalidMessage);
            }
        } else {
            tracing::warn!("Invalid message content");
            return Err(GroupDeletionError::InvalidMessage);
        }

        // Everything seems to be okay.
        // No need to do anything else here, since the group is getting deleted
        // anyway.

        // Finally, we create the message for distribution.
        let c2c_message = processed_assisted_message_plus
            .serialized_mls_message
            .into();

        Ok(c2c_message)
    }
}
