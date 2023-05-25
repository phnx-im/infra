// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage, messages::AssistedMessage,
    openmls::prelude::ProcessedMessageContent,
};
use tls_codec::Deserialize;

use crate::messages::client_ds::{JoinGroupParams, JoinGroupParamsAad, QueueMessagePayload};

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::JoinGroupError,
    group_state::{ClientProfile, DsGroupState, TimeStamp},
};

impl DsGroupState {
    pub(super) fn join_group(
        &mut self,
        params: JoinGroupParams,
    ) -> Result<QueueMessagePayload, JoinGroupError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message =
            if matches!(params.external_commit.message, AssistedMessage::Commit(_)) {
                self.group()
                    .process_assisted_message(params.external_commit.message.clone())
                    .map_err(|_| JoinGroupError::ProcessingError)?
            } else {
                return Err(JoinGroupError::InvalidMessage);
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
                return Err(JoinGroupError::InvalidMessage);
            };

        // The external commit joining the client into the group should contain only the path.
        if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
                || staged_commit.remove_proposals().count() > 0
            {
                return Err(JoinGroupError::InvalidMessage);
            }
        } else {
            return Err(JoinGroupError::InvalidMessage);
        };

        // If there is an AAD, we might have to update the client profile later.
        let aad = JoinGroupParamsAad::tls_deserialize(&mut processed_message.authenticated_data())
            .map_err(|_| JoinGroupError::InvalidMessage)?;

        // Check if the claimed client indices match those in the user's profile.
        if let Some(user_profile) = self.user_profiles.get(&params.sender) {
            if user_profile.clients != aad.existing_user_clients {
                return Err(JoinGroupError::InvalidMessage);
            }
        } else {
            // This should have been checked during validation
            return Err(JoinGroupError::LibraryError);
        }

        // Get the sender's credential s.t. we can identify them later.
        let sender_credential = processed_message.credential().clone();

        // Finalize processing.
        self.group_mut().accept_processed_message(
            processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // Let's figure out the leaf index of the new member.
        let sender = self
            .group()
            .members()
            .find_map(|m| {
                if m.credential == sender_credential {
                    Some(m.index)
                } else {
                    None
                }
            })
            .ok_or(JoinGroupError::ProcessingError)?;

        // Create a client profile and update the user's user profile.
        if let Some(user_profile) = self.user_profiles.get_mut(&params.sender) {
            user_profile.clients.push(sender);
        } else {
            // This should have been checked during validation
            return Err(JoinGroupError::LibraryError);
        }

        let client_profile = ClientProfile {
            leaf_index: sender,
            credential_chain: aad.encrypted_credential_information,
            client_queue_config: params.qs_client_reference,
            activity_time: TimeStamp::now(),
            activity_epoch: self.group().epoch(),
        };
        self.client_profiles.insert(sender, client_profile);

        // Finally, we create the message for distribution.
        let payload = QueueMessagePayload {
            payload: params.external_commit.message_bytes,
        };

        Ok(payload)
    }
}
