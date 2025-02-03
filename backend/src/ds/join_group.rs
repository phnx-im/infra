// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage, messages::SerializedMlsMessage,
    openmls::prelude::ProcessedMessageContent, provider_traits::MlsAssistProvider,
};
use phnxtypes::{
    errors::JoinGroupError,
    messages::client_ds::{InfraAadMessage, InfraAadPayload, JoinGroupParams},
    time::{Duration, TimeStamp},
};
use tls_codec::DeserializeBytes;

use super::{
    group_state::{DsGroupState, MemberProfile},
    process::USER_EXPIRATION_DAYS,
};

impl DsGroupState {
    pub(super) fn join_group(
        &mut self,
        params: JoinGroupParams,
    ) -> Result<SerializedMlsMessage, JoinGroupError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(self.provider.crypto(), params.external_commit)
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
        let aad_message = InfraAadMessage::tls_deserialize_exact_bytes(processed_message.aad())
            .map_err(|_| JoinGroupError::InvalidMessage)?;
        // TODO: Check version of Aad Message
        let aad_payload = if let InfraAadPayload::JoinGroup(aad) = aad_message.into_payload() {
            aad
        } else {
            return Err(JoinGroupError::InvalidMessage);
        };

        // Get the sender's credential s.t. we can identify them later.
        let sender_credential = processed_message.credential().clone();

        // Finalize processing.
        self.group.accept_processed_message(
            self.provider.storage(),
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

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

        let member_profile = MemberProfile {
            leaf_index: sender,
            encrypted_identity_link_key: aad_payload.encrypted_identity_link_key,
            client_queue_config: params.qs_reference,
            activity_time: TimeStamp::now(),
            activity_epoch: self.group().epoch(),
        };
        self.member_profiles.insert(sender, member_profile);

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
