// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::SerializedMlsMessage,
    openmls::prelude::{LeafNodeIndex, ProcessedMessageContent, Sender},
    provider_traits::MlsAssistProvider,
};
use phnxtypes::{
    errors::ClientRemovalError, messages::client_ds::RemoveClientsParams, time::Duration,
};

use super::api::{Provider, USER_EXPIRATION_DAYS};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn remove_clients(
        &mut self,
        provider: &Provider,
        params: RemoveClientsParams,
    ) -> Result<SerializedMlsMessage, ClientRemovalError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(provider.crypto(), params.commit)
            .map_err(|_| ClientRemovalError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                return Err(ClientRemovalError::InvalidMessage);
            };

        // Check if sender index and user profile match.
        let sender_index = if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(ClientRemovalError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                return Err(ClientRemovalError::InvalidMessage);
            };
            leaf_index
        } else {
            // Remove users should be a regular commit
            return Err(ClientRemovalError::InvalidMessage);
        };

        let removed_clients: Vec<LeafNodeIndex> =
            if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
                processed_message.content()
            {
                // Check that the commit only contains removes.
                if staged_commit.add_proposals().count() > 0
                    || staged_commit.update_proposals().count() > 0
                {
                    return Err(ClientRemovalError::InvalidMessage);
                }
                // Process remove proposals, but only non-inline ones.
                let by_reference_removes: Vec<_> = staged_commit
                    .remove_proposals()
                    .filter_map(|remove_proposal| {
                        if let Sender::Member(leaf_index) = remove_proposal.sender() {
                            if leaf_index != sender_index {
                                Some(remove_proposal)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                self.process_referenced_remove_proposals(&by_reference_removes)
                    .map_err(|_| ClientRemovalError::InvalidMessage)?;
                // Let's gather the inline remove proposals. We've already processed the non-inline ones.
                staged_commit
                    .remove_proposals()
                    .filter_map(|remove_proposal| {
                        if let Sender::Member(leaf_index) = remove_proposal.sender() {
                            if leaf_index != sender_index {
                                Some(remove_proposal.remove_proposal().removed())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                return Err(ClientRemovalError::InvalidMessage);
            };

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We first accept the message into the group state ...
        self.group_mut().accept_processed_message(
            provider.storage(),
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        // ... then we update the client profiles and the user profile.
        let user_profile = self
            .user_profiles
            .get_mut(&params.sender)
            // This should have been caught by message validation.
            .ok_or(ClientRemovalError::LibraryError)?;

        // Update the auth key.
        user_profile.user_auth_key = params.new_auth_key;
        for removed_client in removed_clients {
            if let Some(position) = user_profile
                .clients
                .iter()
                .position(|&leaf_index| leaf_index == removed_client)
            {
                user_profile.clients.remove(position);
            } else {
                // The removed client does not seem to belong to this user...
                return Err(ClientRemovalError::InvalidMessage);
            }
            let removed_client = self.client_profiles.remove(&removed_client);
            // Check that we're tracking clients correctly.
            debug_assert!(removed_client.is_some())
        }

        // Finally, we create the message for distribution.
        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
