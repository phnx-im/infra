// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{Group, openmls_provider::PhnxOpenMlsProvider};
use anyhow::{Context, Result, anyhow, bail, ensure};
use mimi_room_policy::{MimiProposal, RoleIndex};
use phnxtypes::{
    credentials::ClientCredential,
    crypto::{
        ear::keys::{EncryptedIdentityLinkKey, EncryptedUserProfileKey},
        indexed_aead::keys::UserProfileKey,
    },
    messages::client_ds::{CredentialUpdate, InfraAadMessage, InfraAadPayload},
};
use sqlx::SqliteConnection;
use tls_codec::DeserializeBytes as TlsDeserializeBytes;
use tracing::info;

use crate::clients::api_clients::ApiClients;

use openmls::prelude::{
    Credential, LeafNodeIndex, ProcessedMessage, ProcessedMessageContent, ProtocolMessage, Sender,
    StagedCommit,
};

use super::client_auth_info::{ClientAuthInfo, GroupMembership};

pub(crate) struct ProcessMessageResult {
    pub(crate) processed_message: ProcessedMessage,
    pub(crate) we_were_removed: bool,
    pub(crate) sender_client_credential: ClientCredential,
    pub(crate) profile_infos: Vec<(ClientCredential, UserProfileKey)>,
}

impl Group {
    /// Process inbound message
    ///
    /// Returns the processed message, whether the group was deleted, as well as
    /// the sender's client credential.
    pub(crate) async fn process_message(
        &mut self,
        connection: &mut SqliteConnection,
        api_clients: &ApiClients,
        message: impl Into<ProtocolMessage>,
    ) -> Result<ProcessMessageResult> {
        // Phase 1: Process the message.
        let processed_message = {
            let provider = PhnxOpenMlsProvider::new(&mut *connection);
            self.mls_group.process_message(&provider, message)?
        };

        let group_id = self.group_id().clone();

        // Will be set to true if we were removed (or the group was deleted).
        let mut we_were_removed = false;
        let mut encrypted_profile_infos: Vec<(ClientCredential, EncryptedUserProfileKey)> =
            Vec::new();
        let sender_index = match processed_message.content() {
            // For now, we only care about commits.
            ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                bail!("Unsupported message type")
            }
            ProcessedMessageContent::ApplicationMessage(_) => {
                info!("Message type: application");
                let sender_client_credential =
                    if let Sender::Member(index) = processed_message.sender() {
                        ClientAuthInfo::load(&mut *connection, &group_id, *index)
                            .await?
                            .map(|info| info.into_client_credential())
                            .context("Could not find client credential of message sender")?
                    } else {
                        bail!("Invalid sender type.")
                    };
                return Ok(ProcessMessageResult {
                    processed_message,
                    we_were_removed,
                    sender_client_credential,
                    profile_infos: Vec::new(),
                });
            }
            ProcessedMessageContent::ProposalMessage(_proposal) => {
                // Proposals are just returned and can then be added to the
                // proposal store after the caller has inspected them.
                let Sender::Member(sender_index) = processed_message.sender() else {
                    bail!("Invalid sender type.")
                };

                // TODO: Room policy checks?

                *sender_index
            }
            ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                let sender_index = match processed_message.sender() {
                    Sender::Member(index) => index.to_owned(),
                    Sender::NewMemberCommit => {
                        self.mls_group.ext_commit_sender_index(staged_commit)?
                    }
                    Sender::External(_) | Sender::NewMemberProposal => {
                        bail!("Invalid sender type.")
                    }
                };

                // StagedCommitMessage Phase 1: Process the proposals.

                // Before we process the AAD payload, we first process the
                // proposals by value. Currently only removes are allowed.
                for remove_proposal in staged_commit.remove_proposals() {
                    let removed_index = remove_proposal.remove_proposal().removed();

                    // Room policy checks
                    self.room_state.apply_regular_proposals(
                        &sender_index.u32(),
                        &[MimiProposal::ChangeRole {
                            target: removed_index.u32(),
                            role: RoleIndex::Outsider,
                        }],
                    )?;

                    GroupMembership::stage_removal(&mut *connection, &group_id, removed_index)
                        .await?;
                    if removed_index == self.mls_group().own_leaf_index() {
                        we_were_removed = true;
                    }
                }

                // Phase 2: Process the AAD payload.
                // Let's figure out which operation this is meant to be.
                let aad_payload =
                    InfraAadMessage::tls_deserialize_exact_bytes(processed_message.aad())?
                        .into_payload();
                match aad_payload {
                    InfraAadPayload::GroupOperation(group_operation_payload) => {
                        // Process adds if there are any.
                        if !group_operation_payload
                            .new_encrypted_user_profile_keys
                            .is_empty()
                        {
                            // Prepare inputs for add processing
                            let added_clients = staged_commit.add_proposals().map(|p| {
                                p.add_proposal()
                                    .key_package()
                                    .leaf_node()
                                    .credential()
                                    .clone()
                            });
                            let client_auth_infos = self
                                .process_adds(
                                    sender_index,
                                    staged_commit,
                                    api_clients,
                                    &mut *connection,
                                    added_clients,
                                )
                                .await?;
                            // Match up client credentials and new UserProfileKeys
                            let new_profile_infos: Vec<_> = client_auth_infos
                                .into_iter()
                                .map(|cai| cai.into_client_credential())
                                .zip(
                                    group_operation_payload
                                        .new_encrypted_user_profile_keys
                                        .into_iter(),
                                )
                                .collect();
                            encrypted_profile_infos.extend(new_profile_infos);
                        }

                        // Process updates if there are any.
                        // Check if the client has updated its leaf credential.
                        let new_sender_credential = staged_commit
                            .update_path_leaf_node()
                            .ok_or(anyhow!("Could not find sender leaf node"))?
                            .credential();

                        if new_sender_credential != processed_message.credential() {
                            self.process_update(
                                api_clients,
                                &mut *connection,
                                new_sender_credential.clone(),
                                group_operation_payload.credential_update_option,
                                sender_index,
                            )
                            .await?;
                        }

                        // Process a resync if this is one
                        if matches!(processed_message.sender(), Sender::NewMemberCommit) {
                            self.process_resync(
                                &processed_message,
                                staged_commit,
                                &mut *connection,
                                sender_index,
                            )
                            .await?;
                        }
                    }
                    InfraAadPayload::Update(update_client_payload) => {
                        // Check if the client has updated its leaf credential.
                        let sender = self
                            .mls_group
                            .members()
                            .find(|m| m.index == sender_index)
                            .ok_or(anyhow!("Could not find sender in group members"))?;
                        let new_sender_credential = staged_commit
                            .update_path_leaf_node()
                            .map(|ln| ln.credential())
                            .ok_or(anyhow!("Could not find sender leaf node"))?;
                        if new_sender_credential != &sender.credential {
                            // If so, then there has to be a new identity link key.
                            let client_auth_info = ClientAuthInfo::decrypt_and_verify(
                                &mut *connection,
                                api_clients,
                                &group_id,
                                self.identity_link_wrapper_key(),
                                sender_index,
                                new_sender_credential.clone(),
                            )
                            .await?;

                            // Persist the updated client auth info.
                            client_auth_info.stage_update(&mut *connection).await?;
                        };
                        // TODO: Validation:
                        // * Check that the sender type fits.
                        // * Check that the client id is the same as before.
                        // * More validation on pseudonymous and client credential?
                    }
                    InfraAadPayload::JoinConnectionGroup(join_connection_group_payload) => {
                        // JoinConnectionGroup Phase 1: Decrypt and verify the
                        // client credential of the joiner
                        let Some(sender_credential) = staged_commit
                            .update_path_leaf_node()
                            .map(|ln| ln.credential().clone())
                        else {
                            bail!("Could not find sender leaf node in staged commit")
                        };

                        let client_auth_info = ClientAuthInfo::decrypt_and_verify(
                            &mut *connection,
                            api_clients,
                            &group_id,
                            &self.identity_link_wrapper_key,
                            sender_index,
                            sender_credential,
                        )
                        .await?;
                        // TODO: (More) validation:
                        // * Check that the user id is unique.
                        // * Check that the proposals fit the operation.
                        // * Check that the sender type fits the operation.
                        // * Check that this group is indeed a connection group.

                        // JoinConnectionGroup Phase 2: Persist the client auth info.
                        client_auth_info.stage_add(&mut *connection).await?;
                        encrypted_profile_infos.push((
                            client_auth_info.into_client_credential(),
                            join_connection_group_payload.encrypted_user_profile_key,
                        ));
                    }
                    InfraAadPayload::Resync => {
                        // TODO: Validation:
                        // * Check that this commit contains exactly one remove proposal
                        // * Check that the sender type is correct (external commit).

                        let sender_credential = staged_commit
                            .update_path_leaf_node()
                            .map(|ln| ln.credential().clone())
                            .context("Could not find sender leaf node in staged commit")?;

                        let removed_index = staged_commit
                            .remove_proposals()
                            .next()
                            .context("Resync operation did not contain a remove proposal")?
                            .remove_proposal()
                            .removed();

                        let mut client_auth_info = ClientAuthInfo::decrypt_credential_and_verify(
                            &mut *connection,
                            api_clients,
                            &group_id,
                            removed_index,
                            sender_credential,
                        )
                        .await?;

                        // Set the client's new leaf index.
                        client_auth_info
                            .group_membership_mut()
                            .set_leaf_index(sender_index);
                        client_auth_info.stage_update(&mut *connection).await?;
                    }
                    InfraAadPayload::DeleteGroup => {
                        we_were_removed = true;
                        // There is nothing else to do at this point.
                    }
                };
                sender_index
            }
        };
        // Get the sender's credential
        // If the sender is added to the group with this commit, we have to load
        // it from the DB with status "staged".

        // Phase 2: Load the sender's client credential.
        let sender_client_credential =
            if matches!(processed_message.sender(), Sender::NewMemberCommit) {
                ClientAuthInfo::load_staged(&mut *connection, &group_id, sender_index).await?
            } else {
                ClientAuthInfo::load(&mut *connection, &group_id, sender_index).await?
            }
            .context("Could not find client credential of message sender")?
            .client_credential()
            .clone()
            .into();

        // Decrypt any user profile keys
        let profile_infos = encrypted_profile_infos
            .into_iter()
            .map(|(client_credential, encrypted_user_profile_key)| {
                let user_profile_key = UserProfileKey::decrypt(
                    self.identity_link_wrapper_key(),
                    &encrypted_user_profile_key,
                    client_credential.identity(),
                )?;
                Ok((client_credential, user_profile_key))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ProcessMessageResult {
            processed_message,
            we_were_removed,
            sender_client_credential,
            profile_infos,
        })
    }

    async fn process_adds(
        &mut self,
        sender_index: LeafNodeIndex,
        staged_commit: &StagedCommit,
        api_clients: &ApiClients,
        connection: &mut SqliteConnection,
        added_clients: impl Iterator<Item = Credential>,
    ) -> Result<Vec<ClientAuthInfo>> {
        // AddUsers Phase 1: Compute the free indices
        let added_clients_with_indices =
            GroupMembership::free_indices(&mut *connection, &self.group_id)
                .await?
                .zip(added_clients.into_iter())
                .collect::<Vec<_>>();

        // Room policy checks
        for client in &added_clients_with_indices {
            self.room_state.apply_regular_proposals(
                &sender_index.u32(),
                &[MimiProposal::ChangeRole {
                    target: client.0.u32(),
                    role: RoleIndex::Regular,
                }],
            )?;
        }

        // AddUsers Phase 2: Decrypt and verify the client credentials.
        let client_auth_infos = ClientAuthInfo::decrypt_and_verify_all(
            &mut *connection,
            api_clients,
            &self.group_id,
            self.identity_link_wrapper_key(),
            added_clients_with_indices.into_iter(),
        )
        .await?;

        // TODO: Validation:
        // * Check that this commit only contains (inline) add proposals
        // * User ids MUST be unique within the group (check both new
        //   and existing credentials for duplicates).
        // * Client IDs MUST be unique within the group (only need to
        //   check new credentials, as client IDs are scoped to user
        //   names).

        // AddUsers Phase 3: Verify and store the client auth infos.
        if staged_commit.add_proposals().count() != client_auth_infos.len() {
            bail!("Number of add proposals and client credentials don't match.")
        }
        // We assume that leaf credentials are in the same order
        // as client credentials.
        for client_auth_info in client_auth_infos.iter() {
            // Persist the client auth info.
            client_auth_info.stage_add(&mut *connection).await?;
        }

        Ok(client_auth_infos)
    }

    async fn process_update(
        &self,
        api_clients: &ApiClients,
        connection: &mut SqliteConnection,
        new_sender_credential: Credential,
        credential_update_option: Option<CredentialUpdate>,
        sender_index: LeafNodeIndex,
    ) -> Result<()> {
        // If so, then there has to be a new identity link key.
        let Some(credential_update) = credential_update_option else {
            bail!("Invalid update client payload.")
        };
        let client_auth_info = ClientAuthInfo::decrypt_and_verify(
            &mut *connection,
            api_clients,
            &self.group_id,
            self.identity_link_wrapper_key(),
            sender_index,
            new_sender_credential,
        )
        .await?;
        // Persist the updated client auth info.
        client_auth_info.stage_update(&mut *connection).await?;

        // TODO: Validation:
        // * Check that the client id is the same as before.

        // Verify a potential new leaf credential.
        Ok(())
    }

    async fn process_resync(
        &self,
        processed_message: &ProcessedMessage,
        staged_commit: &StagedCommit,
        connection: &mut sqlx::SqliteConnection,
        sender_index: LeafNodeIndex,
    ) -> Result<()> {
        let removed_index = staged_commit
            .remove_proposals()
            .next()
            .ok_or(anyhow!(
                "Resync operation did not contain a remove proposal"
            ))?
            .remove_proposal()
            .removed();

        let Some(removed_member) = self.mls_group().member_at(removed_index) else {
            bail!("Could not find removed member in group")
        };

        // Check that the leaf credential hasn't changed during the resync.
        if &removed_member.credential != processed_message.credential() {
            bail!("Invalid resync operation: Leaf credential does not match.")
        }

        let mut client_auth_info = ClientAuthInfo::load(connection, &self.group_id, removed_index)
            .await?
            .ok_or_else(|| anyhow!("Could not find client credential of resync sender"))?;

        // Set the client's new leaf index.
        client_auth_info
            .group_membership_mut()
            .set_leaf_index(sender_index);
        client_auth_info.stage_update(connection).await?;
        Ok(())
    }
}
