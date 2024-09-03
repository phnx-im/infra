// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage, messages::SerializedMlsMessage,
    openmls::prelude::ProcessedMessageContent, openmls_rust_crypto::OpenMlsRustCrypto,
};
use phnxtypes::{
    errors::JoinConnectionGroupError,
    messages::client_ds::{InfraAadMessage, InfraAadPayload, JoinConnectionGroupParams},
    time::{Duration, TimeStamp},
};
use tls_codec::DeserializeBytes;

use super::{
    group_state::{ClientProfile, DsGroupState, UserProfile},
    process::USER_EXPIRATION_DAYS,
};

impl DsGroupState {
    pub(super) fn join_connection_group(
        &mut self,
        provider: &OpenMlsRustCrypto,
        params: JoinConnectionGroupParams,
    ) -> Result<SerializedMlsMessage, JoinConnectionGroupError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(provider, params.external_commit)
            .map_err(|e| {
                tracing::warn!(
                    "Processing error: Could not process assisted message: {:?}",
                    e
                );
                JoinConnectionGroupError::ProcessingError
            })?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                tracing::warn!("Invalid message: Processed message does not contain a commit.");
                return Err(JoinConnectionGroupError::InvalidMessage);
            };

        // The external commit joining the client into the group should contain only the path.
        if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
                || staged_commit.remove_proposals().count() > 0
            {
                return Err(JoinConnectionGroupError::InvalidMessage);
            }
        } else {
            tracing::warn!("Invalid message: External commit contained unexpected proposals.");
            return Err(JoinConnectionGroupError::InvalidMessage);
        };

        let aad_message = InfraAadMessage::tls_deserialize_exact_bytes(processed_message.aad())
            .map_err(|_| {
                tracing::warn!("Invalid message: Failed to deserialize AAD.");
                JoinConnectionGroupError::InvalidMessage
            })?;
        // TODO: Check version of Aad Message
        let aad_payload =
            if let InfraAadPayload::JoinConnectionGroup(aad) = aad_message.into_payload() {
                aad
            } else {
                tracing::warn!("Invalid message: Wrong AAD payload.");
                return Err(JoinConnectionGroupError::InvalidMessage);
            };

        // Check if the group indeed only has one user (prior to the new one joining).
        if self.user_profiles.len() > 1 {
            return Err(JoinConnectionGroupError::NotAConnectionGroup);
        }

        // Get the sender's credential s.t. we can identify them later.
        let sender_credential = processed_message.credential().clone();

        // Finalize processing.
        self.group_mut().accept_processed_message(
            provider,
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        // Let's figure out the leaf index of the new member.
        let sender = if let Some(sender) = self.group().members().find_map(|m| {
            if m.credential == sender_credential {
                Some(m.index)
            } else {
                None
            }
        }) {
            sender
        } else {
            tracing::warn!("Could not find sender in group.");
            return Err(JoinConnectionGroupError::ProcessingError);
        };

        // Create a client profile and a user profile.
        let user_profile = UserProfile {
            clients: vec![sender],
            user_auth_key: params.sender,
        };
        let client_profile = ClientProfile {
            leaf_index: sender,
            encrypted_client_information: aad_payload.encrypted_client_information,
            client_queue_config: params.qs_client_reference,
            activity_time: TimeStamp::now(),
            activity_epoch: self.group().epoch(),
        };

        self.client_profiles.insert(sender, client_profile);

        let hash_collision = self
            .user_profiles
            .insert(user_profile.user_auth_key.hash(), user_profile);
        if hash_collision.is_some() {
            return Err(JoinConnectionGroupError::UserAuthKeyCollision);
        }

        // Finally, we create the message for distribution.
        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
