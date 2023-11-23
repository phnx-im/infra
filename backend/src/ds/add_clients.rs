// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    group::ProcessedAssistedMessage,
    openmls::prelude::{Extension, KeyPackage, OpenMlsProvider, ProcessedMessageContent, Sender},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use phnxtypes::{
    crypto::{
        ear::keys::GroupStateEarKey,
        hpke::{HpkeEncryptable, JoinerInfoEncryptionKey},
    },
    errors::ClientAdditionError,
    identifiers::{QsClientReference, QS_CLIENT_REFERENCE_EXTENSION_TYPE},
    messages::client_ds::{
        AddClientsParams, DsJoinerInformation, InfraAadMessage, InfraAadPayload,
        QsQueueMessagePayload, QsQueueMessageType, WelcomeBundle,
    },
    time::{Duration, TimeStamp},
};
use tls_codec::{DeserializeBytes, Serialize};

use crate::messages::intra_backend::{DsFanOutMessage, DsFanOutPayload};

use super::{api::USER_EXPIRATION_DAYS, group_state::ClientProfile};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn add_clients(
        &mut self,
        params: AddClientsParams,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(DsFanOutPayload, Vec<DsFanOutMessage>), ClientAdditionError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.commit)
            .map_err(|_| ClientAdditionError::ProcessingError)?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let processed_message =
            if let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
                processed_assisted_message_plus.processed_assisted_message
            {
                processed_message
            } else {
                // This should be a commit.
                return Err(ClientAdditionError::InvalidMessage);
            };

        // Validate that the AAD includes enough encrypted credential chains
        let aad_message =
            InfraAadMessage::tls_deserialize_exact(processed_message.authenticated_data())
                .map_err(|_| ClientAdditionError::InvalidMessage)?;
        // TODO: Check version of Aad Message
        let aad_payload =
            if let InfraAadPayload::AddClients(add_clients_aad) = aad_message.into_payload() {
                add_clients_aad
            } else {
                return Err(ClientAdditionError::InvalidMessage);
            };
        let staged_commit = if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            let remove_proposals: Vec<_> = staged_commit.remove_proposals().collect();
            self.process_referenced_remove_proposals(&remove_proposals)
                .map_err(|_| ClientAdditionError::InvalidMessage)?;
            staged_commit
        } else {
            return Err(ClientAdditionError::InvalidMessage);
        };

        // Check if sender index and user profile match.
        if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(ClientAdditionError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                return Err(ClientAdditionError::InvalidMessage);
            };
        }

        // TODO (Spec): We might be able to prove to the DS that we're actually
        // the owning user of the added client(s).

        // A few general checks.
        let number_of_add_proposals = staged_commit.add_proposals().count();
        // Check if we have enough encrypted credential chains.
        if number_of_add_proposals != aad_payload.encrypted_client_information.len() {
            return Err(ClientAdditionError::InvalidMessage);
        }
        let added_clients: Vec<KeyPackage> = staged_commit
            .add_proposals()
            .map(|add_proposal| add_proposal.add_proposal().key_package().clone())
            .collect();

        // Check if for each added member, there is a corresponding entry
        // in the Welcome.
        if added_clients.iter().any(|kp| {
            let key_package_ref = match kp.hash_ref(OpenMlsRustCrypto::default().crypto()) {
                Ok(kp_ref) => kp_ref,
                Err(_) => return true,
            };
            !params
                .welcome
                .joiners()
                .any(|joiner_ref| joiner_ref == key_package_ref)
        }) {
            return Err(ClientAdditionError::IncompleteWelcome);
        }

        // TODO: Verify that the FQDN of all QsClientRefs is the same. We should do the same in add_users.

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We first accept the message into the group state ...
        self.group_mut().accept_processed_message(
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // ... s.t. it's easier to update the user profile.
        let mut fan_out_messages: Vec<DsFanOutMessage> = vec![];
        for (key_package, (encrypted_client_credential, encrypted_signature_ear_key)) in
            added_clients
                .into_iter()
                .zip(aad_payload.encrypted_client_information.into_iter())
        {
            let member = self
                .group()
                .members()
                .find(|m| m.signature_key == key_package.leaf_node().signature_key().as_slice())
                .ok_or(ClientAdditionError::InvalidMessage)?;
            let leaf_index = member.index;

            // Put the client into the user profile.
            let user_profile = self
                .user_profiles
                .get_mut(&params.sender)
                // There has to be a user profile, otherwise authentication would have
                // failed.
                .ok_or(ClientAdditionError::LibraryError)?;
            user_profile.clients.push(leaf_index);

            // Create the client profile.
            let client_queue_config = QsClientReference::tls_deserialize_exact(
                key_package
                    .extensions()
                    .iter()
                    .find_map(|e| match e {
                        Extension::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE, bytes) => {
                            Some(&bytes.0)
                        }
                        _ => None,
                    })
                    .ok_or(ClientAdditionError::MissingQueueConfig)?
                    .as_slice(),
            )
            .map_err(|_| ClientAdditionError::MissingQueueConfig)?;
            let client_profile = ClientProfile {
                leaf_index,
                encrypted_client_information: (
                    encrypted_client_credential,
                    encrypted_signature_ear_key,
                ),
                client_queue_config: client_queue_config.clone(),
                activity_time: TimeStamp::now(),
                activity_epoch: self.group().epoch(),
            };
            // Add the the client profile to the group's client profiles.
            self.client_profiles
                .insert(client_profile.leaf_index, client_profile);
            let info = &[];
            let aad = &[];
            let encryption_key: JoinerInfoEncryptionKey =
                key_package.hpke_init_key().clone().into();
            let encrypted_joiner_info = DsJoinerInformation {
                group_state_ear_key: group_state_ear_key.clone(),
                encrypted_client_credentials: self.client_information(),
                ratchet_tree: self.group().export_ratchet_tree(),
            }
            .encrypt(&encryption_key, info, aad);
            let welcome_bundle = WelcomeBundle {
                welcome: params.welcome.clone(),
                encrypted_attribution_info: params.encrypted_welcome_attribution_infos.clone(),
                encrypted_joiner_info,
            };
            let fan_out_message = DsFanOutMessage {
                payload: DsFanOutPayload::QueueMessage(QsQueueMessagePayload {
                    payload: welcome_bundle
                        .tls_serialize_detached()
                        .map_err(|_| ClientAdditionError::LibraryError)?,
                    message_type: QsQueueMessageType::WelcomeBundle,
                }),
                client_reference: client_queue_config,
            };
            fan_out_messages.push(fan_out_message);
        }

        // Finally, we create the message for distribution.
        let c2c_message = processed_assisted_message_plus
            .serialized_mls_message
            .into();

        Ok((c2c_message, fan_out_messages))
    }
}
