use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage, messages::AssistedMessage, ProcessedMessageContent, Sender,
};
use tls_codec::Deserialize;

use crate::messages::client_ds::{DsFanoutPayload, UpdateClientParams, UpdateClientParamsAad};

use super::{api::USER_EXPIRATION_DAYS, errors::ClientUpdateError, group_state::DsGroupState};

impl DsGroupState {
    pub(super) fn update_client(
        &mut self,
        params: UpdateClientParams,
    ) -> Result<DsFanoutPayload, ClientUpdateError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message =
            if matches!(params.commit.message, AssistedMessage::Commit(_)) {
                self.group()
                    .process_assisted_message(params.commit.message.clone())
                    .map_err(|_| ClientUpdateError::ProcessingError)?
            } else {
                return Err(ClientUpdateError::InvalidMessage);
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
                return Err(ClientUpdateError::InvalidMessage);
            };

        if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
                || staged_commit.remove_proposals().count() > 0
            {
                return Err(ClientUpdateError::InvalidMessage);
            }
        } else {
            return Err(ClientUpdateError::InvalidMessage);
        };

        // Let's retrieve some information before processing the message.
        let sender = if let Sender::Member(sender) = processed_message.sender() {
            *sender
        } else {
            return Err(ClientUpdateError::InvalidMessage);
        };
        let old_sender_credential = self
            .group()
            .leaf(sender)
            .ok_or(ClientUpdateError::UnknownSender)?
            .credential()
            .clone();

        // If there is an AAD, we might have to update the client profile later.
        let aad =
            UpdateClientParamsAad::tls_deserialize(&mut processed_message.authenticated_data())
                .map_err(|_| ClientUpdateError::InvalidMessage)?;

        // Finalize processing.
        self.group_mut().accept_processed_message(
            processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // We update the client profile only if the update has changed the sender's credential.
        let new_sender_credential = self
            .group()
            .leaf(sender)
            .ok_or(ClientUpdateError::UnknownSender)?
            .credential()
            .clone();
        if new_sender_credential != old_sender_credential {
            if let Some(ecc) = aad.option_encrypted_credential_information {
                let client_profile = self
                    .client_profiles
                    .get_mut(&sender)
                    .ok_or(ClientUpdateError::UnknownSender)?;
                client_profile.credential_chain = ecc;
            } else {
                return Err(ClientUpdateError::InvalidMessage);
            }
        }

        // Finally, we create the message for distribution.
        let c2c_message = DsFanoutPayload {
            payload: params.commit.message_bytes,
        };

        Ok(c2c_message)
    }
}
