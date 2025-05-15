// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{Group, openmls_provider::PhnxOpenMlsProvider};
use anyhow::{Context, Result, anyhow, bail};
use phnxtypes::{
    credentials::ClientCredential,
    crypto::ear::keys::EncryptedIdentityLinkKey,
    messages::client_ds::{CredentialUpdate, InfraAadMessage, InfraAadPayload},
};
use sqlx::SqlitePool;
use tls_codec::DeserializeBytes as TlsDeserializeBytes;
use tracing::info;

use crate::clients::api_clients::ApiClients;

use openmls::prelude::{
    Credential, LeafNodeIndex, ProcessedMessage, ProcessedMessageContent, ProtocolMessage, Sender,
    StagedCommit,
};

use super::client_auth_info::{ClientAuthInfo, GroupMembership};

impl Group {
    /// Process inbound message
    ///
    /// Returns the processed message, whether the group was deleted, as well as
    /// the sender's client credential.
    pub(crate) async fn process_message(
        &mut self,
        pool: &SqlitePool,
        api_clients: &ApiClients,
        message: impl Into<ProtocolMessage>,
    ) -> Result<(ProcessedMessage, bool, ClientCredential)> {
        // Phase 1: Process the message.
        let processed_message = {
            let mut connection = pool.acquire().await?;
            let provider = PhnxOpenMlsProvider::new(&mut connection);
            self.mls_group.process_message(&provider, message)?
        };

        let group_id = self.group_id();

        // Will be set to true if we were removed (or the group was deleted).
        let mut we_were_removed = false;
        let sender_index = match processed_message.content() {
            // For now, we only care about commits.
            ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                bail!("Unsupported message type")
            }
            ProcessedMessageContent::ApplicationMessage(_) => {
                info!("Message type: application");
                let sender_client_credential =
                    if let Sender::Member(index) = processed_message.sender() {
                        ClientAuthInfo::load(pool.acquire().await?.as_mut(), group_id, *index)
                            .await?
                            .map(|info| info.into_client_credential())
                            .context("Could not find client credential of message sender")?
                    } else {
                        bail!("Invalid sender type.")
                    };
                return Ok((processed_message, false, sender_client_credential));
            }
            ProcessedMessageContent::ProposalMessage(_proposal) => {
                // Proposals are just returned and can then be added to the
                // proposal store after the caller has inspected them.
                let Sender::Member(sender_index) = processed_message.sender() else {
                    bail!("Invalid sender type.")
                };
                *sender_index
            }
            ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                // StagedCommitMessage Phase 1: Process the proposals.

                // Before we process the AAD payload, we first process the
                // proposals by value. Currently only removes are allowed.
                for remove_proposal in staged_commit.remove_proposals() {
                    let removed_index = remove_proposal.remove_proposal().removed();
                    GroupMembership::stage_removal(pool, group_id, removed_index).await?;
                    if removed_index == self.mls_group().own_leaf_index() {
                        we_were_removed = true;
                    }
                }

                // Phase 2: Process the AAD payload.
                // Let's figure out which operation this is meant to be.
                let aad_payload =
                    InfraAadMessage::tls_deserialize_exact_bytes(processed_message.aad())?
                        .into_payload();
                let sender_index = match processed_message.sender() {
                    Sender::Member(index) => index.to_owned(),
                    Sender::NewMemberCommit => {
                        self.mls_group.ext_commit_sender_index(staged_commit)?
                    }
                    Sender::External(_) | Sender::NewMemberProposal => {
                        bail!("Invalid sender type.")
                    }
                };
                match aad_payload {
                    InfraAadPayload::GroupOperation(group_operation_payload) => {
                        // Process adds if there are any.
                        if !group_operation_payload
                            .new_encrypted_identity_link_keys
                            .is_empty()
                        {
                            // Make sure the vector lengths match.
                            if staged_commit.add_proposals().count()
                                != group_operation_payload
                                    .new_encrypted_identity_link_keys
                                    .len()
                            {
                                bail!(
                                    "Number of add proposals and new identity link keys doesn't match."
                                )
                            }
                            // Prepare inputs for add processing
                            let added_clients = staged_commit
                                .add_proposals()
                                .map(|p| {
                                    p.add_proposal()
                                        .key_package()
                                        .leaf_node()
                                        .credential()
                                        .clone()
                                })
                                .zip(
                                    group_operation_payload
                                        .new_encrypted_identity_link_keys
                                        .into_iter(),
                                );
                            self.process_adds(staged_commit, api_clients, pool, added_clients)
                                .await?;
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
                                pool,
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
                                pool.acquire().await?.as_mut(),
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
                            let Some(encrypted_identity_link_key) =
                                update_client_payload.option_encrypted_identity_link_key
                            else {
                                bail!("Invalid update client payload.")
                            };
                            let client_auth_info = ClientAuthInfo::decrypt_and_verify(
                                pool,
                                api_clients,
                                group_id,
                                self.identity_link_wrapper_key(),
                                encrypted_identity_link_key,
                                sender_index,
                                new_sender_credential.clone(),
                            )
                            .await?;

                            // Persist the updated client auth info.
                            client_auth_info
                                .stage_update(pool.acquire().await?.as_mut())
                                .await?;
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
                            pool,
                            api_clients,
                            group_id,
                            &self.identity_link_wrapper_key,
                            join_connection_group_payload.encrypted_identity_link_key,
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
                        client_auth_info
                            .stage_add(pool.acquire().await?.as_mut())
                            .await?;
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
                        // Get the identity link key of the resyncing client
                        let identity_link_key =
                            GroupMembership::load(pool, group_id, removed_index)
                                .await?
                                .context("Could not find group membership of resync sender")
                                .map(|gm| gm.identity_link_key().clone())?;

                        let mut client_auth_info = ClientAuthInfo::decrypt_credential_and_verify(
                            pool,
                            api_clients,
                            group_id,
                            identity_link_key,
                            removed_index,
                            sender_credential,
                        )
                        .await?;

                        // Set the client's new leaf index.
                        client_auth_info
                            .group_membership_mut()
                            .set_leaf_index(sender_index);
                        client_auth_info
                            .stage_update(pool.acquire().await?.as_mut())
                            .await?;
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
        let mut connection = pool.acquire().await?;
        let sender_client_credential =
            if matches!(processed_message.sender(), Sender::NewMemberCommit) {
                ClientAuthInfo::load_staged(&mut connection, group_id, sender_index).await?
            } else {
                ClientAuthInfo::load(&mut connection, group_id, sender_index).await?
            }
            .context("Could not find client credential of message sender")?
            .client_credential()
            .clone()
            .into();

        Ok((processed_message, we_were_removed, sender_client_credential))
    }

    async fn process_adds(
        &self,
        staged_commit: &StagedCommit,
        api_clients: &ApiClients,
        pool: &SqlitePool,
        added_clients: impl Iterator<Item = (Credential, EncryptedIdentityLinkKey)>,
    ) -> Result<()> {
        // AddUsers Phase 1: Compute the free indices
        let added_clients_with_indices = GroupMembership::free_indices(pool, &self.group_id)
            .await?
            .zip(added_clients.into_iter())
            .map(|(index, (credential, eilk))| ((index, credential), eilk));

        // AddUsers Phase 2: Decrypt and verify the client credentials.
        let client_auth_infos = ClientAuthInfo::decrypt_and_verify_all(
            pool,
            api_clients,
            &self.group_id,
            self.identity_link_wrapper_key(),
            added_clients_with_indices,
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
        {
            let mut connection = pool.acquire().await?;
            for client_auth_info in client_auth_infos.iter() {
                // Persist the client auth info.
                client_auth_info.stage_add(&mut connection).await?;
            }
        }

        Ok(())
    }

    async fn process_update(
        &self,
        api_clients: &ApiClients,
        pool: &SqlitePool,
        new_sender_credential: Credential,
        credential_update_option: Option<CredentialUpdate>,
        sender_index: LeafNodeIndex,
    ) -> Result<()> {
        // If so, then there has to be a new identity link key.
        let Some(credential_update) = credential_update_option else {
            bail!("Invalid update client payload.")
        };
        let client_auth_info = ClientAuthInfo::decrypt_and_verify(
            pool,
            api_clients,
            &self.group_id,
            self.identity_link_wrapper_key(),
            credential_update.encrypted_identity_link_key,
            sender_index,
            new_sender_credential,
        )
        .await?;
        // Persist the updated client auth info.
        client_auth_info
            .stage_update(pool.acquire().await?.as_mut())
            .await?;

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
