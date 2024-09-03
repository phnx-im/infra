// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashSet;

use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::SerializedMlsMessage,
    openmls::prelude::{LeafNodeIndex, ProcessedMessageContent, Sender},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use phnxtypes::{
    crypto::signatures::keys::UserKeyHash, errors::UserRemovalError,
    messages::client_ds::RemoveUsersParams, time::Duration,
};

use super::process::USER_EXPIRATION_DAYS;

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn remove_users(
        &mut self,
        provider: &OpenMlsRustCrypto,
        params: RemoveUsersParams,
    ) -> Result<SerializedMlsMessage, UserRemovalError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(provider, params.commit)
            .map_err(|_| UserRemovalError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                return Err(UserRemovalError::InvalidMessage);
            };

        // Check if sender index and user profile match.
        let sender_index = if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(UserRemovalError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                return Err(UserRemovalError::InvalidMessage);
            };
            leaf_index
        } else {
            // Remove users should be a regular commit
            return Err(UserRemovalError::InvalidMessage);
        };

        let removed_clients: Vec<LeafNodeIndex> =
            if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
                processed_message.content()
            {
                // Check that the commit only contains removes.
                if staged_commit.add_proposals().count() > 0
                    || staged_commit.update_proposals().count() > 0
                {
                    return Err(UserRemovalError::InvalidMessage);
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
                    .map_err(|_| UserRemovalError::InvalidMessage)?;
                // Let's gather the inline remove proposals. We've already processed the non-inline ones.
                staged_commit
                    .remove_proposals()
                    .filter_map(|remove_proposal| {
                        if let Sender::Member(leaf_index) = remove_proposal.sender() {
                            if leaf_index == sender_index {
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
                return Err(UserRemovalError::InvalidMessage);
            };

        // A few general checks.

        // TODO: This should be de-duplicated with how by-reference remove proposals are processed.
        let mut client_profiles_to_be_removed = HashSet::new();
        let mut user_profiles_to_be_removed = Vec::<UserKeyHash>::new();
        for leaf_index in removed_clients.iter() {
            // Check if we've already marked the index as to be removed.
            if !client_profiles_to_be_removed.contains(leaf_index) {
                // If we have not, search for a user that belongs to this index.
                for (user_key_hash, user_profile) in self.user_profiles.iter() {
                    if user_profile.clients.contains(leaf_index) {
                        // Then mark these indices as to be removed indices, ...
                        user_profile.clients.iter().for_each(|index| {
                            client_profiles_to_be_removed.insert(*index);
                        });
                        // and mark the user profile as to be removed.
                        user_profiles_to_be_removed.push(user_key_hash.clone())
                    }
                }
            }
        }

        // Check if all clients of all removed users were indeed removed.
        for marked_index in client_profiles_to_be_removed.iter() {
            if !removed_clients.contains(marked_index) {
                return Err(UserRemovalError::IncompleteRemoval);
            };
        }

        // TODO: Validate that the adder has sufficient privileges (if this
        //       isn't done by an MLS extension).

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // Update the group's user and client profiles.
        for user_key_hash in user_profiles_to_be_removed {
            let removed_user_profile_option = self.user_profiles.remove(&user_key_hash);
            debug_assert!(removed_user_profile_option.is_some());
        }
        for client_index in client_profiles_to_be_removed {
            let removed_client_profile_option = self.client_profiles.remove(&client_index);
            debug_assert!(removed_client_profile_option.is_some());
        }

        // We first accept the message into the group state ...
        self.group_mut().accept_processed_message(
            provider,
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
