// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, Result, bail};
use openmls::{
    group::QueuedProposal,
    prelude::{MlsMessageBodyIn, MlsMessageIn, ProcessedMessageContent, ProtocolMessage, Sender},
};
use phnxtypes::{
    codec::PhnxCodec,
    crypto::ear::EarDecryptable,
    identifiers::AsClientId,
    messages::{
        QueueMessage,
        client_ds::{
            ExtractedQsQueueMessage, ExtractedQsQueueMessagePayload, InfraAadMessage,
            InfraAadPayload, WelcomeBundle,
        },
    },
    time::TimeStamp,
};
use tls_codec::DeserializeBytes;

use crate::{
    ConversationMessage, PartialContact, conversations::ConversationType, groups::Group,
    key_stores::indexed_keys::UserProfileKey,
};

use super::{
    Conversation, ConversationAttributes, ConversationId, CoreUser, FriendshipPackage,
    TimestampedMessage, anyhow,
};
use crate::key_stores::queue_ratchets::StorableQsQueueRatchet;

pub enum ProcessQsMessageResult {
    NewConversation(ConversationId),
    ConversationChanged(ConversationId, Vec<ConversationMessage>),
    ConversationMessages(Vec<ConversationMessage>),
}

pub struct ProcessedQsMessages {
    pub new_conversations: Vec<ConversationId>,
    pub changed_conversations: Vec<ConversationId>,
    pub new_messages: Vec<ConversationMessage>,
}

impl CoreUser {
    /// Decrypt a `QueueMessage` received from the QS queue.
    pub async fn decrypt_qs_queue_message(
        &self,
        qs_message_ciphertext: QueueMessage,
    ) -> Result<ExtractedQsQueueMessage> {
        self.with_transaction(async |connection| {
            let mut qs_queue_ratchet = StorableQsQueueRatchet::load(&mut *connection).await?;
            let payload = qs_queue_ratchet.decrypt(qs_message_ciphertext)?;
            qs_queue_ratchet.update_ratchet(&mut *connection).await?;
            Ok(payload.extract()?)
        })
        .await
    }

    /// Process a decrypted message received from the QS queue.
    ///
    /// Returns the [`ConversationId`] of newly created conversations and any
    /// [`ConversationMessage`]s produced by processin the QS message.
    ///
    /// TODO: This function is (still) async, because depending on the message
    /// it processes, it might do one of the following:
    ///
    /// * fetch credentials from the AS to authenticate existing group members
    ///   (when joining a new group) or new group members (when processing an
    ///   Add or external join)
    /// * download AddInfos (KeyPackages, etc.) from the DS. This happens when a
    ///   user externally joins a connection group and the contact is upgraded
    ///   from partial contact to full contact.
    /// * get a QS verifying key from the QS. This also happens when a user
    ///   externally joins a connection group to verify the KeyPackageBatches
    ///   received from the QS as part of the AddInfo download.
    async fn process_qs_message(
        &self,
        qs_queue_message: ExtractedQsQueueMessage,
    ) -> Result<ProcessQsMessageResult> {
        // TODO: We should verify whether the messages are valid infra messages, i.e.
        // if it doesn't mix requests, etc. I think the DS already does some of this
        // and we might be able to re-use code.

        // Keep track of freshly joined groups s.t. we can later update our user auth keys.
        let ds_timestamp = qs_queue_message.timestamp;
        match qs_queue_message.payload {
            ExtractedQsQueueMessagePayload::WelcomeBundle(welcome_bundle) => {
                self.handle_welcome_bundle(welcome_bundle).await
            }
            ExtractedQsQueueMessagePayload::MlsMessage(mls_message) => {
                self.handle_mls_message(*mls_message, ds_timestamp).await
            }
        }
    }

    async fn handle_welcome_bundle(
        &self,
        welcome_bundle: WelcomeBundle,
    ) -> Result<ProcessQsMessageResult> {
        // WelcomeBundle Phase 1: Join the group. This might involve
        // loading AS credentials or fetching them from the AS.
        let (group, member_profile_info) = Group::join_group(
            welcome_bundle,
            &self.inner.key_store.wai_ear_key,
            self.pool(),
            &self.inner.api_clients,
        )
        .await?;
        let group_id = group.group_id().clone();

        // WelcomeBundle Phase 2: Fetch the user profiles of the group members
        // and decrypt them.

        // TODO: This can fail in some cases. If it does, we should fetch and
        // process messages and then try again.
        let mut user_profiles = Vec::with_capacity(member_profile_info.len());
        for profile_info in member_profile_info {
            let user_profile = self.fetch_user_profile(profile_info).await?;
            user_profiles.push(user_profile);
        }

        // WelcomeBundle Phase 3: Store the user profiles of the group
        // members if they don't exist yet and store the group and the
        // new conversation.
        let conversation_id = self
            .with_transaction_and_notifier(async |connection, notifier| {
                for user_profile in user_profiles {
                    user_profile.store(&mut *connection, notifier).await?;
                }

                // Set the conversation attributes according to the group's
                // group data.
                let group_data = group.group_data().context("No group data")?;
                let attributes: ConversationAttributes = PhnxCodec::from_slice(group_data.bytes())?;

                let conversation =
                    Conversation::new_group_conversation(group_id.clone(), attributes);
                // If we've been in that conversation before, we delete the old
                // conversation (and the corresponding MLS group) first and then
                // create a new one. We do leave the messages intact, though.
                Conversation::delete(&mut *connection, notifier, conversation.id()).await?;
                Group::delete_from_db(connection, &group_id).await?;
                group.store(&mut *connection).await?;
                conversation.store(&mut *connection, notifier).await?;

                Ok(conversation.id())
            })
            .await?;

        Ok(ProcessQsMessageResult::NewConversation(conversation_id))
    }

    async fn handle_mls_message(
        &self,
        mls_message: MlsMessageIn,
        ds_timestamp: TimeStamp,
    ) -> Result<ProcessQsMessageResult> {
        let protocol_message: ProtocolMessage = match mls_message.extract() {
            MlsMessageBodyIn::PublicMessage(handshake_message) =>
                handshake_message.into(),
            // Only application messages are private
            MlsMessageBodyIn::PrivateMessage(app_msg) => app_msg.into(),
            // Welcomes always come as a WelcomeBundle, not as an MLSMessage.
            MlsMessageBodyIn::Welcome(_) |
            // Neither GroupInfos nor KeyPackages should come from the queue.
            MlsMessageBodyIn::GroupInfo(_) | MlsMessageBodyIn::KeyPackage(_) => bail!("Unexpected message type"),
        };
        // MLSMessage Phase 1: Load the conversation and the group.
        let group_id = protocol_message.group_id();
        let conversation = Conversation::load_by_group_id(self.pool(), group_id)
            .await?
            .ok_or_else(|| anyhow!("No conversation found for group ID {:?}", group_id))?;
        let conversation_id = conversation.id();

        let mut connection = self.pool().acquire().await?;
        let mut group = Group::load(&mut connection, group_id)
            .await?
            .ok_or_else(|| anyhow!("No group found for group ID {:?}", group_id))?;
        drop(connection);

        // MLSMessage Phase 2: Process the message
        let (processed_message, we_were_removed, sender_client_id) = group
            .process_message(self.pool(), &self.inner.api_clients, protocol_message)
            .await?;

        let sender = processed_message.sender().clone();
        let aad = processed_message.aad().to_vec();

        // `conversation_changed` indicates whether the state of the conversation was updated
        let (group_messages, conversation_changed) = match processed_message.into_content() {
            ProcessedMessageContent::ApplicationMessage(application_message) => self
                .handle_application_message(application_message, ds_timestamp, &sender_client_id)?,
            ProcessedMessageContent::ProposalMessage(proposal) => {
                self.handle_proposal_message(&mut group, *proposal).await?
            }
            ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                self.handle_staged_commit_message(
                    &mut group,
                    conversation_id,
                    *staged_commit,
                    aad,
                    ds_timestamp,
                    &sender,
                    &sender_client_id,
                    we_were_removed,
                )
                .await?
            }
            ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                self.handle_external_join_proposal_message()?
            }
        };

        // MLSMessage Phase 3: Store the updated group and the messages.
        self.with_transaction_and_notifier(async |connection, notifier| {
            group.store_update(&mut *connection).await?;
            let conversation_messages =
                Self::store_messages(connection, notifier, conversation_id, group_messages).await?;
            Ok(match (conversation_messages, conversation_changed) {
                (messages, true) => {
                    ProcessQsMessageResult::ConversationChanged(conversation_id, messages)
                }
                (messages, false) => ProcessQsMessageResult::ConversationMessages(messages),
            })
        })
        .await
    }

    fn handle_application_message(
        &self,
        application_message: openmls::prelude::ApplicationMessage,
        ds_timestamp: TimeStamp,
        sender_client_id: &AsClientId,
    ) -> anyhow::Result<(Vec<TimestampedMessage>, bool)> {
        let group_messages = vec![TimestampedMessage::from_application_message(
            application_message,
            ds_timestamp,
            sender_client_id.user_name(),
        )];
        Ok((group_messages, false))
    }

    async fn handle_proposal_message(
        &self,
        group: &mut Group,
        proposal: QueuedProposal,
    ) -> anyhow::Result<(Vec<TimestampedMessage>, bool)> {
        // For now, we don't to anything here. The proposal
        // was processed by the MLS group and will be
        // committed with the next commit.
        let mut connection = self.pool().acquire().await?;
        group.store_proposal(&mut connection, proposal)?;
        Ok((vec![], false))
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_staged_commit_message(
        &self,
        group: &mut Group,
        conversation_id: ConversationId,
        staged_commit: openmls::prelude::StagedCommit,
        aad: Vec<u8>,
        ds_timestamp: TimeStamp,
        sender: &openmls::prelude::Sender,
        sender_client_id: &AsClientId,
        we_were_removed: bool,
    ) -> anyhow::Result<(Vec<TimestampedMessage>, bool)> {
        // If a client joined externally, we check if the
        // group belongs to an unconfirmed conversation.

        // StagedCommitMessage Phase 1: Load the conversation.
        let mut conversation = Conversation::load(self.pool(), &conversation_id)
            .await?
            .ok_or_else(|| anyhow!("Can't find conversation with id {}", conversation_id.uuid()))?;
        let mut conversation_changed = false;

        let mut notifier = self.store_notifier();

        if let ConversationType::UnconfirmedConnection(user_name) = conversation.conversation_type()
        {
            // Check if it was an external commit and if the user name matches
            if !matches!(sender, Sender::NewMemberCommit)
                && sender_client_id.user_name() == user_name
            {
                // TODO: Handle the fact that an unexpected user joined the connection group.
            }
            // UnconfirmedConnection Phase 1: Load up the partial contact and decrypt the
            // friendship package
            let partial_contact = PartialContact::load(self.pool(), user_name)
                .await?
                .ok_or_else(|| anyhow!("No partial contact found for user name {}", user_name))?;

            // This is a bit annoying, since we already
            // de-serialized this in the group processing
            // function, but we need the encrypted
            // friendship package here.
            let encrypted_friendship_package =
                if let InfraAadPayload::JoinConnectionGroup(payload) =
                    InfraAadMessage::tls_deserialize_exact_bytes(&aad)?.into_payload()
                {
                    payload.encrypted_friendship_package
                } else {
                    bail!("Unexpected AAD payload")
                };

            let friendship_package = FriendshipPackage::decrypt(
                &partial_contact.friendship_package_ear_key,
                &encrypted_friendship_package,
            )?;

            let user_profile_key = UserProfileKey::from_base_secret(
                friendship_package.user_profile_base_secret.clone(),
                user_name,
            )?;

            // UnconfirmedConnection Phase 2: Fetch the user profile.
            let user_profile = self
                .fetch_user_profile((sender_client_id.clone(), user_profile_key.clone()))
                .await?;

            // Now we can turn the partial contact into a full one.
            partial_contact
                .mark_as_complete(
                    self.pool(),
                    &mut notifier,
                    friendship_package,
                    sender_client_id.clone(),
                    &user_profile,
                    &user_profile_key,
                )
                .await?;

            conversation.confirm(self.pool(), &mut notifier).await?;
            conversation_changed = true;
        }

        // StagedCommitMessage Phase 2: Merge the staged commit into the group.

        // If we were removed, we set the group to inactive.
        if we_were_removed {
            let past_members = group.members(self.pool()).await.into_iter().collect();
            conversation
                .set_inactive(self.pool(), &mut notifier, past_members)
                .await?;
        }
        let mut connection = self.pool().acquire().await?;
        let group_messages = group
            .merge_pending_commit(&mut connection, staged_commit, ds_timestamp)
            .await?;

        notifier.notify();

        Ok((group_messages, conversation_changed))
    }

    fn handle_external_join_proposal_message(
        &self,
    ) -> anyhow::Result<(Vec<TimestampedMessage>, bool)> {
        unimplemented!()
    }

    /// Convenience function that takes a list of `QueueMessage`s retrieved from
    /// the QS, decrypts them, and processes them.
    pub async fn fully_process_qs_messages(
        &self,
        qs_messages: Vec<QueueMessage>,
    ) -> Result<ProcessedQsMessages> {
        // Process each qs message individually
        let mut new_conversations = vec![];
        let mut changed_conversations = vec![];
        let mut new_messages = vec![];
        for qs_message in qs_messages {
            let qs_message_plaintext = self.decrypt_qs_queue_message(qs_message).await?;
            match self.process_qs_message(qs_message_plaintext).await? {
                ProcessQsMessageResult::ConversationMessages(conversation_messages) => {
                    new_messages.extend(conversation_messages);
                }
                ProcessQsMessageResult::ConversationChanged(
                    conversation_id,
                    conversation_messages,
                ) => {
                    new_messages.extend(conversation_messages);
                    changed_conversations.push(conversation_id)
                }
                ProcessQsMessageResult::NewConversation(conversation_id) => {
                    new_conversations.push(conversation_id)
                }
            };
        }

        Ok(ProcessedQsMessages {
            new_conversations,
            changed_conversations,
            new_messages,
        })
    }
}
