// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::{AssistedWelcome, SerializedMlsMessage},
    openmls::{
        group::StagedCommit,
        prelude::{
            Extension, KeyPackage, KeyPackageRef, LeafNodeIndex, OpenMlsProvider,
            ProcessedMessageContent, Sender,
        },
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
    provider_traits::MlsAssistProvider,
};

use phnxtypes::{
    credentials::EncryptedClientCredential,
    crypto::{
        ear::keys::{EncryptedSignatureEarKey, GroupStateEarKey},
        hpke::{HpkeEncryptable, JoinerInfoEncryptionKey},
        signatures::{keys::QsVerifyingKey, signable::Verifiable},
    },
    errors::GroupOperationError,
    identifiers::{Fqdn, QsClientReference, QS_CLIENT_REFERENCE_EXTENSION_TYPE},
    keypackage_batch::{KeyPackageBatch, KEYPACKAGEBATCH_EXPIRATION, VERIFIED},
    messages::{
        client_ds::{
            AddUsersInfo, DsJoinerInformation, GroupOperationParams, GroupOperationParamsAad,
            InfraAadMessage, InfraAadPayload, WelcomeBundle,
        },
        welcome_attribution_info::EncryptedWelcomeAttributionInfo,
    },
    time::{Duration, TimeStamp},
};
use tls_codec::DeserializeBytes;

use crate::{
    messages::intra_backend::{DsFanOutMessage, DsFanOutPayload},
    qs::QsConnector,
};

use super::{group_state::ClientProfile, process::USER_EXPIRATION_DAYS};

use super::group_state::DsGroupState;

#[derive(Clone, Copy)]
enum SenderIndex {
    Member(LeafNodeIndex),
    External(LeafNodeIndex),
}

impl SenderIndex {
    fn leaf_index(&self) -> LeafNodeIndex {
        match self {
            SenderIndex::Member(leaf_index) => *leaf_index,
            SenderIndex::External(leaf_index) => *leaf_index,
        }
    }
}

impl DsGroupState {
    // TODO: Structured logging
    // TODO: Make into a sans-io-style state machine
    pub(crate) async fn group_operation<Q: QsConnector>(
        &mut self,
        params: GroupOperationParams,
        group_state_ear_key: &GroupStateEarKey,
        qs_provider: &Q,
    ) -> Result<(SerializedMlsMessage, Vec<DsFanOutMessage>), GroupOperationError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message_plus = self
            .group()
            .process_assisted_message(self.provider.crypto(), params.commit)
            .map_err(|e| {
                tracing::warn!("Error processing assisted message: {:?}", e);
                GroupOperationError::ProcessingError
            })?;

        // Perform DS-level validation
        // Make sure that we have the right message type.
        let ProcessedAssistedMessage::Commit(ref processed_message, ref _group_info) =
            &processed_assisted_message_plus.processed_assisted_message
        else {
            // This should be a commit.
            return Err(GroupOperationError::InvalidMessage);
        };

        // Validate that the AAD includes enough encrypted credential chains
        let aad_message = InfraAadMessage::tls_deserialize_exact_bytes(processed_message.aad())
            .map_err(|_| GroupOperationError::InvalidMessage)?;
        // TODO: Check version of Aad Message
        let InfraAadPayload::GroupOperation(aad_payload) = aad_message.into_payload() else {
            return Err(GroupOperationError::InvalidMessage);
        };

        // Extract the message's content
        let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        else {
            return Err(GroupOperationError::InvalidMessage);
        };

        // Perform validation depending on the type of message
        let sender_index = match processed_message.sender() {
            Sender::Member(leaf_index) => SenderIndex::Member(*leaf_index),
            Sender::NewMemberCommit => {
                // If it's an external commit, it has to be a resync operation,
                // which means there MUST be a remove proposal for the sender's
                // original client. That client MUST be removed in the commit
                // and that client MUST have a user profile associated with it.
                let Some(remove_proposal) = staged_commit.remove_proposals().next() else {
                    return Err(GroupOperationError::InvalidMessage);
                };
                SenderIndex::External(remove_proposal.remove_proposal().removed())
            }
            // A group operation must be a commit.
            Sender::External(_) | Sender::NewMemberProposal => {
                return Err(GroupOperationError::InvalidMessage);
            }
        };

        if !self
            .user_profiles
            .get(&params.sender)
            .ok_or(GroupOperationError::LibraryError)?
            .clients
            .contains(&sender_index.leaf_index())
        {
            return Err(GroupOperationError::InvalidMessage);
        };

        // Check if the operation adds a user.
        let adds_users = staged_commit.add_proposals().count() != 0;

        // TODO: Validate that the senders of the proposals have sufficient
        //       privileges (if this isn't done by an MLS extension). Note that
        //       we have to check the sender of the proposals not those of the
        //       commit.

        // Validation related to adding users
        let added_users_state_option = if !adds_users {
            None
        } else {
            let Some(add_users_info) = params.add_users_info_option else {
                return Err(GroupOperationError::InvalidMessage);
            };

            let mut verifying_keys = HashMap::new();

            // Fetch all verifying keys for the domains of the added users.
            // TODO: There should be some DS-level caching of verifying keys.
            // For now, we fetch them each time.
            for domain in add_users_info
                .key_package_batches
                .iter()
                .map(|kpb| kpb.homeserver_domain())
            {
                let verifying_key = qs_provider
                    .verifying_key(domain.clone())
                    .await
                    .map_err(|_| GroupOperationError::FailedToObtainVerifyingKey)?;
                verifying_keys.insert(domain.clone(), verifying_key);
            }

            let add_users_state =
                validate_added_users(staged_commit, &aad_payload, add_users_info, verifying_keys)?;
            Some(add_users_state)
        };

        // Validation related to resync operations
        let external_sender_information = match sender_index {
            SenderIndex::External(original_index) => {
                // Make sure there is a remove proposal for the original client.
                if staged_commit.remove_proposals().count() == 0 {
                    return Err(GroupOperationError::InvalidMessage);
                }
                // Collect the encrypted client information and the client queue
                // config of the original client. We need this later to create
                // the new client profile.
                let encrypted_client_information = self
                    .client_profiles
                    .get(&original_index)
                    .ok_or(GroupOperationError::InvalidMessage)?
                    .encrypted_client_information
                    .clone();
                // Get the queue config from the leaf node extensions.
                let client_queue_config = staged_commit
                    .update_path_leaf_node()
                    .ok_or(GroupOperationError::InvalidMessage)?
                    .extensions()
                    .iter()
                    .find_map(|e| match e {
                        Extension::Unknown(QS_CLIENT_REFERENCE_EXTENSION_TYPE, ref bytes) => {
                            let extension =
                                QsClientReference::tls_deserialize_exact_bytes(&bytes.0)
                                    .map_err(|_| GroupOperationError::InvalidMessage);
                            Some(extension)
                        }
                        _ => None,
                    })
                    .ok_or(GroupOperationError::InvalidMessage)??;
                Some((encrypted_client_information, client_queue_config))
            }
            _ => None,
        };

        let removed_clients = staged_commit
            .remove_proposals()
            .map(|remove_proposal| remove_proposal.remove_proposal().removed())
            .collect::<Vec<_>>();

        // Everything seems to be okay.
        // Now we have to update the group state and distribute.

        // We first accept the message into the group state ...
        self.group.accept_processed_message(
            self.provider.storage(),
            processed_assisted_message_plus.processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        )?;

        // Process removes

        self.remove_profiles(removed_clients, sender_index);

        // ... s.t. it's easier to update the user and client profiles.

        let mut fan_out_messages: Vec<DsFanOutMessage> = vec![];

        // If users were added, update the list of user profiles
        if let Some(AddUsersState {
            added_users,
            welcome,
        }) = added_users_state_option
        {
            self.update_membership_profiles(&added_users)?;
            fan_out_messages.extend(self.generate_fan_out_messages(
                added_users,
                group_state_ear_key,
                &welcome,
            )?);
        }

        // Process resync operations
        if let Some((encrypted_client_information, client_queue_config)) =
            external_sender_information
        {
            // The original client profile was already removed. We just have to
            // add the new one.
            let client_profile = ClientProfile {
                leaf_index: sender_index.leaf_index(),
                encrypted_client_information,
                client_queue_config,
                activity_time: TimeStamp::now(),
                activity_epoch: self.group().epoch(),
            };
            self.client_profiles
                .insert(sender_index.leaf_index(), client_profile);
        }

        // Finally, we create the message for distribution.
        Ok((
            processed_assisted_message_plus.serialized_mls_message,
            fan_out_messages,
        ))
    }

    /// Updates client and user profiles based on the added users.
    fn update_membership_profiles(
        &mut self,
        added_users: &[(
            Vec<(
                KeyPackage,
                (EncryptedClientCredential, EncryptedSignatureEarKey),
            )>,
            EncryptedWelcomeAttributionInfo,
        )],
    ) -> Result<(), GroupOperationError> {
        for (add_packages, _) in added_users.iter() {
            let mut client_profiles = vec![];
            for (key_package, encrypted_client_information) in add_packages {
                let member = self
                    .group()
                    .members()
                    .find(|m| m.signature_key == key_package.leaf_node().signature_key().as_slice())
                    .ok_or(GroupOperationError::InvalidMessage)?;
                let leaf_index = member.index;
                let client_queue_config = QsClientReference::tls_deserialize_exact_bytes(
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
                        .ok_or(GroupOperationError::MissingQueueConfig)?
                        .as_slice(),
                )
                .map_err(|_| GroupOperationError::MissingQueueConfig)?;
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

        Ok(())
    }

    fn generate_fan_out_messages(
        &self,
        added_users: Vec<(
            Vec<(
                KeyPackage,
                (EncryptedClientCredential, EncryptedSignatureEarKey),
            )>,
            EncryptedWelcomeAttributionInfo,
        )>,
        group_state_ear_key: &GroupStateEarKey,
        welcome: &AssistedWelcome,
    ) -> Result<Vec<DsFanOutMessage>, GroupOperationError> {
        let mut fan_out_messages = vec![];
        for (add_packages, attribution_info) in added_users.into_iter() {
            for (key_package, _) in add_packages {
                let client_queue_config = QsClientReference::tls_deserialize_exact_bytes(
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
                        .ok_or(GroupOperationError::MissingQueueConfig)?
                        .as_slice(),
                )
                .map_err(|_| GroupOperationError::MissingQueueConfig)?;
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
                    welcome: welcome.clone(),
                    encrypted_attribution_info: attribution_info.clone(),
                    encrypted_joiner_info,
                };
                let fan_out_message = DsFanOutMessage {
                    payload: DsFanOutPayload::QueueMessage(
                        welcome_bundle
                            .try_into()
                            .map_err(|_| GroupOperationError::LibraryError)?,
                    ),
                    client_reference: client_queue_config,
                };
                fan_out_messages.push(fan_out_message);
            }
        }

        Ok(fan_out_messages)
    }

    /// Removes user and client profiles based on the list of removed clients.
    // TODO: Simplified version as we don't support multi-client yet.
    fn remove_profiles(&mut self, removed_clients: Vec<LeafNodeIndex>, sender_index: SenderIndex) {
        for client_index in removed_clients {
            let removed_client_profile_option = self.client_profiles.remove(&client_index);
            debug_assert!(removed_client_profile_option.is_some());

            // Don't remove user profiles for external senders as they remain in
            // the group.
            if let SenderIndex::External(_) = sender_index {
                continue;
            }

            let marked_users = self
                .user_profiles
                .iter()
                .filter_map(|(user_key_hash, user_profile)| {
                    if user_profile.clients.contains(&client_index) {
                        Some(user_key_hash.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            for user_key_hash in marked_users.iter() {
                let removed_user_profile_option = self.user_profiles.remove(user_key_hash);
                debug_assert!(removed_user_profile_option.is_some());
            }
        }
    }
}

struct AddUsersState {
    added_users: Vec<(
        Vec<(
            KeyPackage,
            (EncryptedClientCredential, EncryptedSignatureEarKey),
        )>,
        EncryptedWelcomeAttributionInfo,
    )>,
    welcome: AssistedWelcome,
}

fn validate_added_users(
    staged_commit: &StagedCommit,
    aad_payload: &GroupOperationParamsAad,
    add_users_info: AddUsersInfo,
    verifying_keys: HashMap<Fqdn, QsVerifyingKey>,
) -> Result<AddUsersState, GroupOperationError> {
    let number_of_added_users = staged_commit.add_proposals().count();
    // Check that the lengths of the various vectors match.
    if aad_payload.new_encrypted_credential_information.len() != number_of_added_users
        || add_users_info.encrypted_welcome_attribution_infos.len() != number_of_added_users
        || add_users_info.key_package_batches.len() != number_of_added_users
    {
        return Err(GroupOperationError::InvalidMessage);
    }

    // Collect all added clients in a map s.t. we can later check if all
    // added clients are present in the Welcome.
    let mut added_clients: HashMap<
        KeyPackageRef,
        (
            KeyPackage,
            (EncryptedClientCredential, EncryptedSignatureEarKey),
        ),
    > = staged_commit
        .add_proposals()
        .zip(aad_payload.new_encrypted_credential_information.iter())
        .map(|(add_proposal, (ecc, esek))| {
            let key_package_ref = add_proposal
                .add_proposal()
                .key_package()
                .hash_ref(OpenMlsRustCrypto::default().crypto())
                .map_err(|_| GroupOperationError::LibraryError)?;
            let key_package = add_proposal.add_proposal().key_package().clone();
            Ok((key_package_ref, (key_package, (ecc.clone(), esek.clone()))))
        })
        .collect::<Result<_, GroupOperationError>>()?;

    // Check if for each added member, there is a corresponding entry
    // in the Welcome.
    if added_clients.iter().any(|(add_proposal_ref, _)| {
        !add_users_info
            .welcome
            .joiners()
            .any(|joiner_ref| &joiner_ref == add_proposal_ref)
    }) {
        return Err(GroupOperationError::IncompleteWelcome);
    }

    // Verify all KeyPackageBatches.
    let mut added_users = vec![];
    for (key_package_batch, attribution_info) in add_users_info.key_package_batches.into_iter().zip(
        add_users_info
            .encrypted_welcome_attribution_infos
            .into_iter(),
    ) {
        let fqdn = key_package_batch.homeserver_domain().clone();

        let Some(verifying_key) = verifying_keys.get(&fqdn) else {
            // All verifying keys should be present in the map.
            return Err(GroupOperationError::LibraryError);
        };

        let key_package_batch: KeyPackageBatch<VERIFIED> =
            key_package_batch.verify(verifying_key).map_err(|e| {
                tracing::warn!(
                    "Error verifying key package batch with pre-fetched key: {:?}",
                    e
                );
                GroupOperationError::InvalidKeyPackageBatch
            })?;

        // Validate freshness of the batch.
        if key_package_batch.has_expired(KEYPACKAGEBATCH_EXPIRATION) {
            tracing::warn!("Key package batch has expired");
            return Err(GroupOperationError::InvalidKeyPackageBatch);
        }

        let mut key_packages = vec![];
        // Check if the KeyPackages in each batch are all present in the commit.
        for key_package_ref in key_package_batch.key_package_refs() {
            let Some(added_client) = added_clients.remove(key_package_ref) else {
                tracing::warn!("Incomplete KeyPackageBatch");
                return Err(GroupOperationError::InvalidKeyPackageBatch);
            };
            // Also, let's store the signature keys s.t. we can later find the
            // KeyPackages belonging to one user in the tree.
            key_packages.push(added_client);
        }
        added_users.push((key_packages, attribution_info));
    }

    Ok(AddUsersState {
        added_users,
        welcome: add_users_info.welcome,
    })
}
