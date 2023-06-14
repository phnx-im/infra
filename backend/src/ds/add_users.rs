// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage,
    messages::{AssistedMessage, AssistedWelcome},
    openmls::prelude::Extension,
    openmls::prelude::{
        KeyPackage, KeyPackageRef, OpenMlsCryptoProvider, ProcessedMessageContent, Sender,
    },
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use tls_codec::{
    Deserialize as TlsDeserializeTrait, Serialize, TlsDeserialize, TlsSerialize, TlsSize,
};

use crate::{
    crypto::{ear::keys::GroupStateEarKey, signatures::signable::Verifiable, EncryptionPublicKey},
    messages::{
        client_ds::{AddUsersParams, AddUsersParamsAad, QueueMessagePayload},
        intra_backend::{DsFanOutMessage, DsFanOutPayload},
    },
    qs::{
        Fqdn, KeyPackageBatchTbs, QsClientReference, QsConnector, QsVerifyingKey,
        KEYPACKAGEBATCH_EXPIRATION_DAYS,
    },
};

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::UserAdditionError,
    group_state::{ClientProfile, EncryptedCredentialChain, TimeStamp},
};

use super::group_state::DsGroupState;

#[derive(TlsSerialize, TlsDeserialize, TlsSize, Clone)]
pub struct WelcomeBundle {
    pub welcome: AssistedWelcome,
    pub encrypted_attribution_info: Vec<u8>,
    pub encrypted_group_state_ear_key: Vec<u8>,
}

impl DsGroupState {
    pub(crate) async fn add_users<Q: QsConnector>(
        &mut self,
        params: AddUsersParams,
        group_state_ear_key: &GroupStateEarKey,
        qs_provider: &Q,
    ) -> Result<(DsFanOutPayload, Vec<DsFanOutMessage>), UserAdditionError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message =
            if matches!(params.commit.message, AssistedMessage::Commit(_)) {
                self.group()
                    .process_assisted_message(params.commit.message.clone())
                    .map_err(|_| UserAdditionError::ProcessingError)?
            } else {
                return Err(UserAdditionError::InvalidMessage);
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
                return Err(UserAdditionError::InvalidMessage);
            };

        // Validate that the AAD includes enough encrypted credential chains
        let aad = AddUsersParamsAad::tls_deserialize(&mut processed_message.authenticated_data())
            .map_err(|_| UserAdditionError::InvalidMessage)?;
        let staged_commit = if let ProcessedMessageContent::StagedCommitMessage(staged_commit) =
            processed_message.content()
        {
            let remove_proposals: Vec<_> = staged_commit.remove_proposals().collect();
            self.process_referenced_remove_proposals(&remove_proposals)
                .map_err(|_| UserAdditionError::InvalidMessage)?;
            staged_commit
        } else {
            return Err(UserAdditionError::InvalidMessage);
        };

        // Check if sender index and user profile match.
        if let Sender::Member(leaf_index) = processed_message.sender() {
            // There should be a user profile. If there wasn't, verification should have failed.
            if !self
                .user_profiles
                .get(&params.sender)
                .ok_or(UserAdditionError::LibraryError)?
                .clients
                .contains(leaf_index)
            {
                return Err(UserAdditionError::InvalidMessage);
            };
        }

        // Check if we have enough encrypted credential chains.
        if staged_commit.add_proposals().count() != aad.encrypted_credential_information.len() {
            return Err(UserAdditionError::InvalidMessage);
        }
        let mut added_clients: HashMap<KeyPackageRef, (KeyPackage, EncryptedCredentialChain)> =
            staged_commit
                .add_proposals()
                .zip(aad.encrypted_credential_information.into_iter())
                .map(|(add_proposal, ecc)| {
                    let key_package_ref = add_proposal
                        .add_proposal()
                        .key_package()
                        .hash_ref(OpenMlsRustCrypto::default().crypto())
                        .map_err(|_| UserAdditionError::LibraryError)?;
                    let key_package = add_proposal.add_proposal().key_package().clone();
                    Ok((key_package_ref, (key_package, ecc)))
                })
                .collect::<Result<
                    HashMap<KeyPackageRef, (KeyPackage, EncryptedCredentialChain)>,
                    UserAdditionError,
                >>()?;

        // Check if for each added member, there is a corresponding entry
        // in the Welcome.
        if added_clients.iter().any(|(add_proposal_ref, _)| {
            !params
                .welcome
                .joiners()
                .any(|joiner_ref| &joiner_ref == add_proposal_ref)
        }) {
            return Err(UserAdditionError::IncompleteWelcome);
        }

        // Verify all KeyPackageBatches.
        let mut verifying_keys: HashMap<Fqdn, QsVerifyingKey> = HashMap::new();
        let mut added_users = vec![];
        // Check that we have enough welcome attribution infos.
        if params.key_package_batches.len() != params.encrypted_welcome_attribution_infos.len() {
            return Err(UserAdditionError::InvalidMessage);
        }
        for (key_package_batch, attribution_info) in params
            .key_package_batches
            .into_iter()
            .zip(params.encrypted_welcome_attribution_infos.into_iter())
        {
            let fqdn = key_package_batch.homeserver_domain().clone();
            let key_package_batch: KeyPackageBatchTbs =
                if let Some(verifying_key) = verifying_keys.get(&fqdn) {
                    key_package_batch
                        .verify(verifying_key)
                        .map_err(|_| UserAdditionError::InvalidKeyPackageBatch)?
                } else {
                    let verifying_key = qs_provider
                        .verifying_key(&fqdn)
                        .await
                        .map_err(|_| UserAdditionError::FailedToObtainVerifyingKey)?;
                    let kpb = key_package_batch
                        .verify(&verifying_key)
                        .map_err(|_| UserAdditionError::InvalidKeyPackageBatch)?;
                    verifying_keys.insert(fqdn, verifying_key);
                    kpb
                };

            // Validate freshness of the batch.
            if key_package_batch.has_expired(KEYPACKAGEBATCH_EXPIRATION_DAYS) {
                return Err(UserAdditionError::InvalidKeyPackageBatch);
            }

            let mut key_packages = vec![];
            // Check if the KeyPackages in each batch are all present in the commit.
            for key_package_ref in key_package_batch.key_package_refs() {
                if let Some(added_client) = added_clients.remove(key_package_ref) {
                    // Also, let's store the signature keys s.t. we can later find the
                    // KeyPackages belonging to one user in the tree.
                    key_packages.push(added_client);
                } else {
                    return Err(UserAdditionError::InvalidKeyPackageBatch);
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
            processed_assisted_message,
            Duration::days(USER_EXPIRATION_DAYS),
        );

        // ... s.t. it's easier to update the user and client profiles.
        let mut fan_out_messages: Vec<DsFanOutMessage> = vec![];
        for (add_packages, attribution_info) in added_users.into_iter() {
            let mut client_profiles = vec![];
            for (key_package, ecc) in add_packages {
                let member = self
                    .group()
                    .members()
                    .find(|m| m.signature_key == key_package.leaf_node().signature_key().as_slice())
                    .ok_or(UserAdditionError::InvalidMessage)?;
                let leaf_index = member.index;
                let client_queue_config = QsClientReference::tls_deserialize(
                    &mut key_package
                        .extensions()
                        .iter()
                        .find_map(|e| match e {
                            Extension::Unknown(0xff00, ref bytes) => Some(&bytes.0),
                            _ => None,
                        })
                        .ok_or(UserAdditionError::MissingQueueConfig)?
                        .as_slice(),
                )
                .map_err(|_| UserAdditionError::MissingQueueConfig)?;
                let client_profile = ClientProfile {
                    leaf_index,
                    credential_chain: ecc,
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
                    .map_err(|_| UserAdditionError::LibraryError)?;
                let welcome_bundle = WelcomeBundle {
                    welcome: params.welcome.clone(),
                    encrypted_attribution_info: attribution_info.clone(),
                    encrypted_group_state_ear_key: encrypted_ear_key
                        .tls_serialize_detached()
                        .map_err(|_| UserAdditionError::LibraryError)?,
                };
                let fan_out_message = DsFanOutMessage {
                    payload: DsFanOutPayload::QueueMessage(QueueMessagePayload {
                        payload: welcome_bundle
                            .tls_serialize_detached()
                            .map_err(|_| UserAdditionError::LibraryError)?,
                    }),
                    client_reference: client_queue_config,
                };
                fan_out_messages.push(fan_out_message);
                client_profiles.push(client_profile);
            }
            let clients = client_profiles.iter().map(|cp| cp.leaf_index).collect();
            // TODO: Make sure that we check that users are put into the user
            // profile map when they first add a user auth key.
            self.unmerged_users.push(clients);
            for client_profile in client_profiles.into_iter() {
                self.client_profiles
                    .insert(client_profile.leaf_index, client_profile);
            }
        }

        // Finally, we create the message for distribution.
        let c2c_message = DsFanOutPayload::QueueMessage(QueueMessagePayload {
            payload: params.commit.message_bytes,
        });

        Ok((c2c_message, fan_out_messages))
    }
}
