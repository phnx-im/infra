// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage,
    openmls::prelude::{ProcessedMessageContent, Proposal, Sender},
};

use crate::messages::{client_ds::SelfRemoveClientParams, intra_backend::DsFanOutPayload};

use super::{api::USER_EXPIRATION_DAYS, errors::ClientSelfRemovalError};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn self_remove_client(
        &mut self,
        params: SelfRemoveClientParams,
    ) -> Result<DsFanOutPayload, ClientSelfRemovalError> {
        // Process message (but don't apply it yet). This performs
        // mls-assist-level validations and puts the proposal into mls-assist's
        // proposal store.
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.remove_proposal)
            .map_err(|_| ClientSelfRemovalError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message = if let ProcessedAssistedMessage::NonCommit(ref processed_message) =
            &processed_assisted_message_plus.processed_assisted_message
        {
            processed_message
        } else {
            // This should be a commit.
            return Err(ClientSelfRemovalError::InvalidMessage);
        };

        // Check if sender index and user profile match.
        let sender_index = if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(ClientSelfRemovalError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                return Err(ClientSelfRemovalError::InvalidMessage);
            };
            *leaf_index
        } else {
            // The remove proposal should come from a member.
            return Err(ClientSelfRemovalError::InvalidMessage);
        };

        if let ProcessedMessageContent::ProposalMessage(queued_proposal) =
            processed_message.content()
        {
            // Check that the commit only contains removes.
            if let Proposal::Remove(remove_proposal) = queued_proposal.proposal() {
                if remove_proposal.removed() != sender_index {
                    return Err(ClientSelfRemovalError::InvalidMessage);
                }
            } else {
                return Err(ClientSelfRemovalError::InvalidMessage);
            }
        } else {
            return Err(ClientSelfRemovalError::InvalidMessage);
        };

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We first accept the message into the group state ...
        self.group_mut().accept_processed_message(
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // We remove the user and client profile only when the proposal is committed.

        // Finally, we create the message for distribution.
        let payload = processed_assisted_message_plus
            .serialized_mls_message
            .into();

        Ok(payload)
    }
}
