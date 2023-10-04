// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{group::ProcessedAssistedMessage, openmls::prelude::ProcessedMessageContent};
use phnx_types::{
    messages::client_ds::{InfraAadMessage, InfraAadPayload, JoinGroupParams},
    time::{Duration, TimeStamp},
};
use tls_codec::DeserializeBytes;

use crate::messages::intra_backend::DsFanOutPayload;

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::JoinGroupError,
    group_state::{ClientProfile, DsGroupState},
};

impl DsGroupState {
    pub(super) fn join_group(
        &mut self,
        params: JoinGroupParams,
    ) -> Result<DsFanOutPayload, JoinGroupError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.external_commit)
            .map_err(|_| JoinGroupError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
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
        let aad_message =
            InfraAadMessage::tls_deserialize_exact(processed_message.authenticated_data())
                .map_err(|_| JoinGroupError::InvalidMessage)?;
        // TODO: Check version of Aad Message
        let aad_payload = if let InfraAadPayload::JoinGroup(aad) = aad_message.into_payload() {
            aad
        } else {
            return Err(JoinGroupError::InvalidMessage);
        };
        // Check if the claimed client indices match those in the user's profile.
        if let Some(user_profile) = self.user_profiles.get(&params.sender) {
            if user_profile.clients != aad_payload.existing_user_clients {
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
            processed_assisted_message_plus.processed_assisted_message,
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
            encrypted_client_information: aad_payload.encrypted_client_information,
            client_queue_config: params.qs_client_reference,
            activity_time: TimeStamp::now(),
            activity_epoch: self.group().epoch(),
        };
        self.client_profiles.insert(sender, client_profile);

        // Finally, we create the message for distribution.
        let payload = processed_assisted_message_plus
            .serialized_mls_message
            .into();

        Ok(payload)
    }
}
