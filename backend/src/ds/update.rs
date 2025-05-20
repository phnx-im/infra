// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::{AssistedMessageIn, SerializedMlsMessage},
    openmls::prelude::{ProcessedMessageContent, Sender},
    provider_traits::MlsAssistProvider,
};
use phnxtypes::{
    messages::client_ds::{InfraAadMessage, InfraAadPayload},
    time::Duration,
};
use tls_codec::DeserializeBytes;

use crate::errors::ClientUpdateError;

use super::{group_state::DsGroupState, process::USER_EXPIRATION_DAYS};

impl DsGroupState {
    pub(super) fn update_client(
        &mut self,
        commit: AssistedMessageIn,
    ) -> Result<SerializedMlsMessage, ClientUpdateError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(self.provider.crypto(), commit)
            .map_err(|_| ClientUpdateError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(processed_message, _group_info) =
                &processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                tracing::warn!("Invalid message type");
                return Err(ClientUpdateError::InvalidMessage);
            };

        if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            if staged_commit.add_proposals().count() > 0
                || staged_commit.update_proposals().count() > 0
            {
                tracing::warn!("Client update message contained add or update proposals");
                return Err(ClientUpdateError::InvalidMessage);
            }
        } else {
            tracing::warn!("Client update message was not a commit");
            return Err(ClientUpdateError::InvalidMessage);
        };

        // Let's retrieve some information before processing the message.
        let sender = if let Sender::Member(sender) = processed_message.sender() {
            *sender
        } else {
            tracing::warn!("Invalid sender type");
            return Err(ClientUpdateError::InvalidMessage);
        };
        let old_sender_credential = self
            .group()
            .leaf(sender)
            .ok_or(ClientUpdateError::UnknownSender)?
            .credential()
            .clone();

        // If there is an AAD, we might have to update the client profile later.
        let aad_message = InfraAadMessage::tls_deserialize_exact_bytes(processed_message.aad())
            .map_err(|_| {
                tracing::warn!("Error deserializing AAD payload");
                ClientUpdateError::InvalidMessage
            })?;
        // TODO: Check version of Aad Message
        let aad_payload = if let InfraAadPayload::Update(aad) = aad_message.into_payload() {
            aad
        } else {
            tracing::warn!("Invalid AAD payload");
            return Err(ClientUpdateError::InvalidMessage);
        };

        // Finalize processing.
        self.group.accept_processed_message(
            self.provider.storage(),
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        // We update the client profile only if the update has changed the sender's credential.
        let new_sender_credential = self
            .group()
            .leaf(sender)
            .ok_or(ClientUpdateError::UnknownSender)?
            .credential()
            .clone();
        if new_sender_credential != old_sender_credential {
            if let Some(encrypted_identity_link_key) =
                aad_payload.option_encrypted_identity_link_key
            {
                let client_profile = self
                    .member_profiles
                    .get_mut(&sender)
                    .ok_or(ClientUpdateError::UnknownSender)?;
                client_profile.encrypted_identity_link_key = encrypted_identity_link_key;
            } else {
                tracing::warn!("No encrypted signature EAR key in AAD payload");
                return Err(ClientUpdateError::InvalidMessage);
            }
        }

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
