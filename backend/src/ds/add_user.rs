use std::collections::HashMap;

use chrono::Duration;
use mls_assist::{
    group::ProcessedAssistedMessage, messages::AssistedMessage, KeyPackage, KeyPackageRef,
    OpenMlsCryptoProvider, OpenMlsRustCrypto, ProcessedMessageContent,
};
use tls_codec::Deserialize as TlsDeserializeTrait;

use crate::{
    crypto::signatures::{keys::QsVerifyingKey, signable::Verifiable},
    messages::client_ds::{AddUsersParams, AddUsersParamsAad, ClientToClientMsg},
    qs::{Fqdn, KeyPackageBatchTbs, QsClientReference, KEYPACKAGEBATCH_EXPIRATION_DAYS},
};

use super::{
    api::USER_EXPIRATION_DAYS,
    errors::UserAdditionError,
    group_state::{ClientProfile, EncryptedCredentialChain, TimeStamp},
};

use super::group_state::DsGroupState;

impl DsGroupState {
    // TODO: Revisit how we process the Identities in the KeyPackages here.
    pub(crate) fn add_users(
        &mut self,
        params: AddUsersParams,
    ) -> Result<ClientToClientMsg, UserAdditionError> {
        // Process message (but don't apply it yet). This performs mls-assist-level validations.
        let processed_assisted_message = if matches!(params.commit, AssistedMessage::Commit(_)) {
            self.group()
                .process_assisted_message(params.commit.clone())
                .map_err(|_| UserAdditionError::ProcessingError)?
        } else {
            return Err(UserAdditionError::InvalidMessage);
        };

        // Perform DS-level validation
        // TODO: Verify that the added clients belong to one user. This requires
        // us to define the credentials we're using. To do that, we'd need to
        // modify OpenMLS.

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
            staged_commit
        } else {
            return Err(UserAdditionError::InvalidMessage);
        };

        // A few general checks.
        let number_of_add_proposals = staged_commit.add_proposals().count();
        // Check if we have enough encrypted WelcomeAttributionInfos.
        if number_of_add_proposals != params.encrypted_welcome_attribution_infos.len() {
            return Err(UserAdditionError::InvalidMessage);
        }
        // Check if we have enough encrypted credential chains.
        if number_of_add_proposals != aad.encrypted_credential_information.len() {
            return Err(UserAdditionError::InvalidMessage);
        }
        let mut added_clients: HashMap<KeyPackageRef, KeyPackage> = staged_commit
            .add_proposals()
            .map(|add_proposal| {
                let key_package_ref = add_proposal
                    .add_proposal()
                    .key_package()
                    .hash_ref(OpenMlsRustCrypto::default().crypto())
                    .map_err(|_| UserAdditionError::LibraryError)?;
                let key_package = add_proposal.add_proposal().key_package().clone();
                Ok((key_package_ref, key_package))
            })
            .collect::<Result<HashMap<KeyPackageRef, KeyPackage>, UserAdditionError>>()?;

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
        for key_package_batch in params.key_package_batches {
            let fqdn = key_package_batch.homeserver_domain().clone();
            let key_package_batch: KeyPackageBatchTbs =
                if let Some(verifying_key) = verifying_keys.get(&fqdn) {
                    key_package_batch
                        .verify(verifying_key)
                        .map_err(|_| UserAdditionError::InvalidKeyPackageBatch)?
                } else {
                    let verifying_key = self
                        .get_qs_verifying_key(&fqdn)
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

            let mut key_packages: Vec<KeyPackage> = vec![];
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
            added_users.push(key_packages);
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
        for key_packages in added_users.into_iter() {
            let client_profiles = key_packages
                .into_iter()
                .filter_map(|kp| {
                    self.group()
                        .members()
                        .find(|m| m.signature_key == kp.leaf_node().signature_key().as_slice())
                        .map(|m| {
                            let leaf_index = m.index;
                            let client_queue_config = QsClientReference::tls_deserialize(
                                &mut kp
                                    .extensions()
                                    .queue_config()
                                    .ok_or(UserAdditionError::MissingQueueConfig)?
                                    .payload(),
                            )
                            .map_err(|_| UserAdditionError::MissingQueueConfig)?;
                            Ok(ClientProfile {
                                leaf_index,
                                credential_chain: EncryptedCredentialChain {},
                                client_queue_config,
                                activity_time: TimeStamp::now(),
                                activity_epoch: self.group().epoch(),
                            })
                        })
                })
                .collect::<Result<Vec<ClientProfile>, UserAdditionError>>()?;
            let clients = client_profiles.iter().map(|cp| cp.leaf_index).collect();
            self.unmerged_users.push(clients);
            for client_profile in client_profiles.into_iter() {
                self.client_profiles
                    .insert(client_profile.leaf_index, client_profile);
            }
        }

        // Finally, we distribute the message.
        let c2c_message = ClientToClientMsg {
            assisted_message: params.commit_bytes,
        };

        Ok(c2c_message)
    }
}
