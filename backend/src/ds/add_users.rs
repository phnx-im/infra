// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage,
    openmls::prelude::Extension,
    openmls::prelude::{
        KeyPackage, KeyPackageRef, OpenMlsCryptoProvider, ProcessedMessageContent, Sender,
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use tls_codec::DeserializeBytes;

use crate::{
    crypto::{
        ear::keys::{EncryptedSignatureEarKey, GroupStateEarKey},
        hpke::{HpkeEncryptable, JoinerInfoEncryptionKey},
        signatures::signable::Verifiable,
    },
    messages::{
        client_ds::{
            AddUsersParams, DsJoinerInformation, InfraAadMessage, InfraAadPayload, WelcomeBundle,
        },
        intra_backend::{DsFanOutMessage, DsFanOutPayload},
    },
    qs::{
        Fqdn, KeyPackageBatch, QsClientReference, QsConnector, QsVerifyingKey,
        KEYPACKAGEBATCH_EXPIRATION_DAYS, VERIFIED,
    },
};

use super::{
    api::{QS_CLIENT_REFERENCE_EXTENSION_TYPE, USER_EXPIRATION_DAYS},
    errors::AddUsersError,
    group_state::{ClientProfile, EncryptedClientCredential, TimeStamp},
};

use super::group_state::DsGroupState;

impl DsGroupState {
    pub(crate) async fn add_users<Q: QsConnector>(
        &mut self,
        params: AddUsersParams,
        group_state_ear_key: &GroupStateEarKey,
        qs_provider: &Q,
    ) -> Result<(DsFanOutPayload, Vec<DsFanOutMessage>), AddUsersError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(params.commit)
            .map_err(|e| {
                tracing::warn!("Error processing assisted message: {:?}", e);
                AddUsersError::ProcessingError
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
                return Err(AddUsersError::InvalidMessage);
            };

        // Validate that the AAD includes enough encrypted credential chains
        let aad_message =
            InfraAadMessage::tls_deserialize_exact(processed_message.authenticated_data())
                .map_err(|_| AddUsersError::InvalidMessage)?;
        // TODO: Check version of Aad Message
        let aad_payload = if let InfraAadPayload::AddUsers(aad) = aad_message.into_payload() {
            aad
        } else {
            return Err(AddUsersError::InvalidMessage);
        };
        let staged_commit = if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            let remove_proposals: Vec<_> = staged_commit.remove_proposals().collect();
            self.process_referenced_remove_proposals(&remove_proposals)
                .map_err(|_| AddUsersError::InvalidMessage)?;
            staged_commit
        } else {
            return Err(AddUsersError::InvalidMessage);
        };

        // Check if sender index and user profile match.
        if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(AddUsersError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                return Err(AddUsersError::InvalidMessage);
            };
        }

        // Check if we have enough encrypted credential chains.
        if staged_commit.add_proposals().count()
            != aad_payload.encrypted_credential_information.len()
        {
            return Err(AddUsersError::InvalidMessage);
        }
        let mut added_clients: HashMap<
            KeyPackageRef,
            (
                KeyPackage,
                (EncryptedClientCredential, EncryptedSignatureEarKey),
            ),
        > = staged_commit
            .add_proposals()
            .zip(aad_payload.encrypted_credential_information.into_iter())
            .map(|(add_proposal, (ecc, esek))| {
                let key_package_ref = add_proposal
                    .add_proposal()
                    .key_package()
                    .hash_ref(OpenMlsRustCrypto::default().crypto())
                    .map_err(|_| AddUsersError::LibraryError)?;
                let key_package = add_proposal.add_proposal().key_package().clone();
                Ok((key_package_ref, (key_package, (ecc, esek))))
            })
            .collect::<Result<
                HashMap<
                    KeyPackageRef,
                    (
                        KeyPackage,
                        (EncryptedClientCredential, EncryptedSignatureEarKey),
                    ),
                >,
                AddUsersError,
            >>()?;

        // Check if for each added member, there is a corresponding entry
        // in the Welcome.
        if added_clients.iter().any(|(add_proposal_ref, _)| {
            !params
                .welcome
                .joiners()
                .any(|joiner_ref| &joiner_ref == add_proposal_ref)
        }) {
            return Err(AddUsersError::IncompleteWelcome);
        }

        // Verify all KeyPackageBatches.
        let mut verifying_keys: HashMap<Fqdn, QsVerifyingKey> = HashMap::new();
        let mut added_users = vec![];
        // Check that we have enough welcome attribution infos.
        if params.key_package_batches.len() != params.encrypted_welcome_attribution_infos.len() {
            return Err(AddUsersError::InvalidMessage);
        }
        for (key_package_batch, attribution_info) in params
            .key_package_batches
            .into_iter()
            .zip(params.encrypted_welcome_attribution_infos.into_iter())
        {
            let fqdn = key_package_batch.homeserver_domain().clone();
            let key_package_batch: KeyPackageBatch<VERIFIED> =
                if let Some(verifying_key) = verifying_keys.get(&fqdn) {
                    key_package_batch
                        .verify(verifying_key)
                        .map_err(|_| AddUsersError::InvalidKeyPackageBatch)?
                } else {
                    let verifying_key = qs_provider
                        .verifying_key(&fqdn)
                        .await
                        .map_err(|_| AddUsersError::FailedToObtainVerifyingKey)?;
                    let kpb = key_package_batch
                        .verify(&verifying_key)
                        .map_err(|_| AddUsersError::InvalidKeyPackageBatch)?;
                    verifying_keys.insert(fqdn, verifying_key);
                    kpb
                };

            // Validate freshness of the batch.
            if key_package_batch.has_expired(KEYPACKAGEBATCH_EXPIRATION_DAYS) {
                return Err(AddUsersError::InvalidKeyPackageBatch);
            }

            let mut key_packages = vec![];
            // Check if the KeyPackages in each batch are all present in the commit.
            for key_package_ref in key_package_batch.key_package_refs() {
                if let Some(added_client) = added_clients.remove(key_package_ref) {
                    // Also, let's store the signature keys s.t. we can later find the
                    // KeyPackages belonging to one user in the tree.
                    key_packages.push(added_client);
                } else {
                    return Err(AddUsersError::InvalidKeyPackageBatch);
                }
            }
            added_users.push((key_packages, attribution_info));
        }

        // TODO: Validate that the adder has sufficient privileges (if this
        //       isn't done by an MLS extension).

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We first accept the message into the group state ...
        self.group_mut().accept_processed_message(
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // ... s.t. it's easier to update the user and client profiles.

        // We have to make two passes over the added users s.t. we can first
        // update the client profiles and the compile the information for the
        // joiner based on the updated client profiles.
        for (add_packages, _) in added_users.iter() {
            let mut client_profiles = vec![];
            for (key_package, encrypted_client_information) in add_packages {
                let member = self
                    .group()
                    .members()
                    .find(|m| m.signature_key == key_package.leaf_node().signature_key().as_slice())
                    .ok_or(AddUsersError::InvalidMessage)?;
                let leaf_index = member.index;
                let client_queue_config = QsClientReference::tls_deserialize_exact(
                    key_package
                        .leaf_node()
                        .extensions()
                        .iter()
                        .find_map(|e| match e {
                            Extension::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE, ref bytes) => {
                                Some(&bytes.0)
                            }
                            _ => None,
                        })
                        .ok_or(AddUsersError::MissingQueueConfig)?
                        .as_slice(),
                )
                .map_err(|_| AddUsersError::MissingQueueConfig)?;
                let client_profile = ClientProfile {
                    leaf_index,
                    encrypted_client_information: encrypted_client_information.clone(),
                    client_queue_config: client_queue_config.clone(),
                    activity_time: TimeStamp::now(),
                    activity_epoch: self.group().epoch(),
                };
                client_profiles.push(client_profile);
            }
            let clients = client_profiles.iter().map(|cp| cp.leaf_index).collect();
            self.unmerged_users.push(clients);
            for client_profile in client_profiles.into_iter() {
                self.client_profiles
                    .insert(client_profile.leaf_index, client_profile);
            }
        }
        let mut fan_out_messages: Vec<DsFanOutMessage> = vec![];
        for (add_packages, attribution_info) in added_users.into_iter() {
            for (key_package, _) in add_packages {
                let client_queue_config = QsClientReference::tls_deserialize_exact(
                    key_package
                        .leaf_node()
                        .extensions()
                        .iter()
                        .find_map(|e| match e {
                            Extension::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE, ref bytes) => {
                                Some(&bytes.0)
                            }
                            _ => None,
                        })
                        .ok_or(AddUsersError::MissingQueueConfig)?
                        .as_slice(),
                )
                .map_err(|_| AddUsersError::MissingQueueConfig)?;
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
                    encrypted_attribution_info: attribution_info.clone(),
                    encrypted_joiner_info,
                };
                let fan_out_message = DsFanOutMessage {
                    payload: DsFanOutPayload::QueueMessage(
                        welcome_bundle
                            .try_into()
                            .map_err(|_| AddUsersError::LibraryError)?,
                    ),
                    client_reference: client_queue_config,
                };
                fan_out_messages.push(fan_out_message);
            }
        }

        // Finally, we create the message for distribution.
        let c2c_message = processed_assisted_message_plus
            .serialized_mls_message
            .into();

        Ok((c2c_message, fan_out_messages))
    }
}
