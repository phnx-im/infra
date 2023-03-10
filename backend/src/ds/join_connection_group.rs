use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage, messages::AssistedMessage, ProcessedMessageContent,
};
use tls_codec::Deserialize;

use crate::messages::client_ds::{
    DsFanoutPayload, JoinConnectionGroupParams, JoinConnectionGroupParamsAad,
};

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::JoinConnectionGroupError,
    group_state::{ClientProfile, DsGroupState, TimeStamp, UserProfile},
};

impl DsGroupState {
    pub(super) fn join_connection_group(
        &mut self,
        params: JoinConnectionGroupParams,
    ) -> Result<DsFanoutPayload, JoinConnectionGroupError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message =
            if matches!(params.external_commit.message, AssistedMessage::Commit(_)) {
                self.group()
                    .process_assisted_message(params.external_commit.message.clone())
                    .map_err(|_| JoinConnectionGroupError::ProcessingError)?
            } else {
                return Err(JoinConnectionGroupError::InvalidMessage);
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
            return Err(JoinConnectionGroupError::InvalidMessage);
        };

        let aad = JoinConnectionGroupParamsAad::tls_deserialize(
            &mut processed_message.authenticated_data(),
        )
        .map_err(|_| JoinConnectionGroupError::InvalidMessage)?;

        // Check if the group indeed only has one user (prior to the new one joining).
        if self.user_profiles.len() > 1 {
            return Err(JoinConnectionGroupError::NotAConnectionGroup);
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
            .ok_or(JoinConnectionGroupError::ProcessingError)?;

        // Create a client profile and a user profile.
        let user_profile = UserProfile {
            clients: vec![sender],
            user_auth_key: params.sender,
        };
        let client_profile = ClientProfile {
            leaf_index: sender,
            credential_chain: aad.encrypted_credential_information,
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
        let payload = DsFanoutPayload {
            payload: params.external_commit.message_bytes,
        };

        Ok(payload)
    }
}
