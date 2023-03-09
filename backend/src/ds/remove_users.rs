use std::collections::HashSet;

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage, messages::AssistedMessage, LeafNodeIndex,
    ProcessedMessageContent,
};

use crate::messages::client_ds::{ClientToClientMsg, RemoveUsersParams};

use super::{api::USER_EXPIRATION_DAYS, errors::UserRemovalError, group_state::UserKeyHash};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn remove_users(
        &mut self,
        params: RemoveUsersParams,
    ) -> Result<ClientToClientMsg, UserRemovalError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message =
            if matches!(params.commit.commit, AssistedMessage::Commit(_)) {
                self.group()
                    .process_assisted_message(params.commit.commit.clone())
                    .map_err(|_| UserRemovalError::ProcessingError)?
            } else {
                return Err(UserRemovalError::InvalidMessage);
            };

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                return Err(UserRemovalError::InvalidMessage);
            };

        let staged_commit = if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            staged_commit
        } else {
            return Err(UserRemovalError::InvalidMessage);
        };

        // A few general checks.
        let removed_clients: Vec<LeafNodeIndex> = staged_commit
            .remove_proposals()
            .map(|remove_proposal| remove_proposal.remove_proposal().removed())
            .collect();

        let mut client_profiles_to_be_removed = HashSet::new();
        let mut user_profiles_to_be_removed = Vec::<UserKeyHash>::new();
        for leaf_index in removed_clients.iter() {
            // Check if we've already marked the index as to be removed.
            if client_profiles_to_be_removed.contains(leaf_index) {
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
            self.user_profiles.remove(&user_key_hash);
        }
        for client_index in client_profiles_to_be_removed {
            self.client_profiles.remove(&client_index);
        }

        // We first accept the message into the group state ...
        self.group_mut().accept_processed_message(
            processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // Finally, we create the message for distribution.
        let c2c_message = ClientToClientMsg {
            assisted_message: params.commit.commit_bytes,
        };

        Ok(c2c_message)
    }
}
