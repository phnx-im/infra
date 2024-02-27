// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::SerializedMlsMessage,
    openmls::prelude::{ProcessedMessageContent, Sender},
};
use phnxtypes::{
    errors::ClientUpdateError,
    messages::client_ds::{InfraAadMessage, InfraAadPayload, UpdateClientParams},
    time::Duration,
};
use tls_codec::DeserializeBytes;

use super::{
    api::USER_EXPIRATION_DAYS,
    group_state::{DsGroupState, UserProfile},
};

impl DsGroupState {
    pub(super) fn update_client(
        &mut self,
        params: UpdateClientParams,
    ) -> Result<SerializedMlsMessage, ClientUpdateError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.commit)
            .map_err(|_| ClientUpdateError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
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
            let remove_proposals: Vec<_> = staged_commit.remove_proposals().collect();
            self.process_referenced_remove_proposals(&remove_proposals)
                .map_err(|e| {
                    tracing::warn!("Error processing referenced remove proposals: {:?}", e);
                    ClientUpdateError::InvalidMessage
                })?;
        } else {
            tracing::warn!("Client update message was not acommit");
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
        let aad_message =
            InfraAadMessage::tls_deserialize_exact_bytes(processed_message.authenticated_data())
                .map_err(|_| {
                    tracing::warn!("Error deserializing AAD payload");
                    ClientUpdateError::InvalidMessage
                })?;
        // TODO: Check version of Aad Message
        let aad_payload = if let InfraAadPayload::UpdateClient(aad) = aad_message.into_payload() {
            aad
        } else {
            tracing::warn!("Invalid AAD payload");
            return Err(ClientUpdateError::InvalidMessage);
        };

        // Finalize processing.
        self.group_mut().accept_processed_message(
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        if let Some(user_auth_key) = params.new_user_auth_key_option {
            let user_key_hash = user_auth_key.hash();
            // Let's figure out if the sending user has not yet set its user auth key.
            if let Some(position) = self
                .unmerged_users
                .iter()
                .position(|clients| clients.contains(&sender))
            {
                let unmerged_user_clients = self.unmerged_users.remove(position);
                let user_profile = UserProfile {
                    clients: unmerged_user_clients,
                    user_auth_key: user_auth_key.clone(),
                };
                if self
                    .user_profiles
                    .insert(user_key_hash, user_profile)
                    .is_some()
                {
                    // We have a user auth key collision
                    tracing::warn!("Unmerged user tries to set duplicate user auth key");
                    return Err(ClientUpdateError::InvalidMessage);
                };
            } else {
                self.user_profiles
                    .get_mut(&user_key_hash)
                    // There has to be a valid user profile since the user is
                    // not unmerged.
                    .ok_or({
                        tracing::warn!("Could not find user profile of sending client.");
                        ClientUpdateError::InvalidMessage
                    })?
                    .user_auth_key = user_auth_key.clone();
            }
        }

        // We update the client profile only if the update has changed the sender's credential.
        let new_sender_credential = self
            .group()
            .leaf(sender)
            .ok_or(ClientUpdateError::UnknownSender)?
            .credential()
            .clone();
        if new_sender_credential != old_sender_credential {
            if let Some(encrypted_signature_ear_key) =
                aad_payload.option_encrypted_signature_ear_key
            {
                let client_profile = self
                    .client_profiles
                    .get_mut(&sender)
                    .ok_or(ClientUpdateError::UnknownSender)?;
                client_profile.encrypted_client_information.1 = encrypted_signature_ear_key;
            } else {
                tracing::warn!("No encrypted signature EAR key in AAD payload");
                return Err(ClientUpdateError::InvalidMessage);
            }
            if let Some(ecc) = aad_payload.option_encrypted_client_credential {
                let client_profile = self
                    .client_profiles
                    .get_mut(&sender)
                    .ok_or(ClientUpdateError::UnknownSender)?;
                client_profile.encrypted_client_information.0 = ecc;
            }
        }

        Ok(processed_assisted_message_plus.serialized_mls_message)
    }
}
