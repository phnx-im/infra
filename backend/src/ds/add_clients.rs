// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::AssistedMessage,
    openmls::prelude::{
        Extension, KeyPackage, OpenMlsCryptoProvider, ProcessedMessageContent, Sender,
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use tls_codec::{Deserialize as TlsDeserializeTrait, Serialize};

use crate::{
    crypto::{ear::keys::GroupStateEarKey, EncryptionPublicKey},
    messages::{
        client_ds::{
            AddClientsParams, AddClientsParamsAad, QueueMessagePayload, QueueMessageType,
            WelcomeBundle,
        },
        intra_backend::DsFanOutMessage,
    },
    qs::QsClientReference,
};

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::ClientAdditionError,
    group_state::{ClientProfile, TimeStamp},
};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) fn add_clients(
        &mut self,
        params: AddClientsParams,
        group_state_ear_key: &GroupStateEarKey,
    ) -> Result<(QueueMessagePayload, Vec<DsFanOutMessage>), ClientAdditionError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message =
            if matches!(params.commit.message, AssistedMessage::Commit(_)) {
                self.group()
                    .process_assisted_message(params.commit.message.clone())
                    .map_err(|_| ClientAdditionError::ProcessingError)?
            } else {
                return Err(ClientAdditionError::InvalidMessage);
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
                return Err(ClientAdditionError::InvalidMessage);
            };

        // Validate that the AAD includes enough encrypted credential chains
        let aad = AddClientsParamsAad::tls_deserialize(&mut processed_message.authenticated_data())
            .map_err(|_| ClientAdditionError::InvalidMessage)?;
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
        if number_of_add_proposals != aad.encrypted_credential_information.len() {
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
            processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // ... s.t. it's easier to update the user profile.
        let mut fan_out_messages: Vec<DsFanOutMessage> = vec![];
        for (key_package, encrypted_client_credential) in added_clients
            .into_iter()
            .zip(aad.encrypted_credential_information.into_iter())
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
            let client_queue_config = QsClientReference::tls_deserialize(
                &mut key_package
                    .extensions()
                    .iter()
                    .find_map(|e| match e {
                        Extension::Unknown(0xff00, bytes) => Some(&bytes.0),
                        _ => None,
                    })
                    .ok_or(ClientAdditionError::MissingQueueConfig)?
                    .as_slice(),
            )
            .map_err(|_| ClientAdditionError::MissingQueueConfig)?;
            let client_profile = ClientProfile {
                leaf_index,
                encrypted_client_credential,
                client_queue_config: client_queue_config.clone(),
                activity_time: TimeStamp::now(),
                activity_epoch: self.group().epoch(),
            };
            // TODO: We should do this nicely via a trait at some point.
            let info = [
                "GroupStateEarKey ".as_bytes(),
                self.group()
                    .group_info()
                    .group_context()
                    .group_id()
                    .as_slice(),
            ]
            .concat();
            let encryption_key_bytes: Vec<u8> = key_package.hpke_init_key().clone().into();
            let encrypted_ear_key = EncryptionPublicKey::from(encryption_key_bytes)
                .encrypt(&info, &[], group_state_ear_key.as_slice())
                .map_err(|_| ClientAdditionError::LibraryError)?;
            let welcome_bundle = WelcomeBundle {
                welcome: params.welcome.clone(),
                encrypted_attribution_info: params.encrypted_welcome_attribution_infos.clone(),
                encrypted_joiner_info: encrypted_ear_key
                    .tls_serialize_detached()
                    .map_err(|_| ClientAdditionError::LibraryError)?,
            };
            let fan_out_message = DsFanOutMessage {
                payload: QueueMessagePayload {
                    payload: welcome_bundle
                        .tls_serialize_detached()
                        .map_err(|_| ClientAdditionError::LibraryError)?,
                    message_type: QueueMessageType::WelcomeBundle,
                },
                client_reference: client_queue_config,
            };
            // Add the the client profile to the group's client profiles.
            self.client_profiles
                .insert(client_profile.leaf_index, client_profile);
            fan_out_messages.push(fan_out_message);
        }

        // Finally, we create the message for distribution.
        let c2c_message = params.commit.message_bytes.into();

        Ok((c2c_message, fan_out_messages))
    }
}
