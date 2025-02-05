// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{bail, Context, Result};
use openmls::{
    group::QueuedProposal,
    prelude::{MlsMessageBodyIn, MlsMessageIn, ProcessedMessageContent, ProtocolMessage, Sender},
};
use phnxtypes::{
    codec::PhnxCodec,
    crypto::ear::EarDecryptable,
    identifiers::AsClientId,
    messages::{
        client_ds::{
            ExtractedQsQueueMessage, ExtractedQsQueueMessagePayload, InfraAadMessage,
            InfraAadPayload, WelcomeBundle,
        },
        QueueMessage,
    },
    time::TimeStamp,
};
use tls_codec::DeserializeBytes;

use crate::{conversations::ConversationType, groups::Group, ConversationMessage, PartialContact};

use super::{
    anyhow, Asset, Conversation, ConversationAttributes, ConversationId, CoreUser,
    FriendshipPackage, TimestampedMessage, UserProfile,
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
        let mut connection = self.inner.connection.lock().await;
        let transaction = connection.transaction()?;
        let mut qs_queue_ratchet = StorableQsQueueRatchet::load(&transaction)?;

        let payload = qs_queue_ratchet.decrypt(qs_message_ciphertext)?;

        qs_queue_ratchet.update_ratchet(&transaction)?;
        transaction.commit()?;

        Ok(payload.extract()?)
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
        let group = Group::join_group(
            welcome_bundle,
            &self.inner.key_store.wai_ear_key,
            self.inner.connection.clone(),
            &self.inner.api_clients,
        )
        .await?;
        let group_id = group.group_id().clone();

        // WelcomeBundle Phase 2: Store the user profiles of the group
        // members if they don't exist yet and store the group and the
        // new conversation.
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;
        let mut notifier = self.store_notifier();
        group
            .members(&transaction)
            .into_iter()
            .try_for_each(|user_name| {
                UserProfile::new(user_name, None, None).store(&transaction, &mut notifier)
            })?;

        // Set the conversation attributes according to the group's
        // group data.
        let group_data = group.group_data().context("No group data")?;
        let attributes: ConversationAttributes = PhnxCodec::from_slice(group_data.bytes())?;

        let conversation = Conversation::new_group_conversation(group_id.clone(), attributes);
        // If we've been in that conversation before, we delete the old
        // conversation (and the corresponding MLS group) first and then
        // create a new one. We do leave the messages intact, though.
        Conversation::delete(&transaction, &mut notifier, conversation.id())?;
        Group::delete_from_db(&mut transaction, &group_id)?;
        group.store(&transaction)?;
        conversation.store(&transaction, &mut notifier)?;
        transaction.commit()?;
        notifier.notify();

        Ok(ProcessQsMessageResult::NewConversation(conversation.id()))
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
        let connection = self.inner.connection.lock().await;
        let conversation = Conversation::load_by_group_id(&connection, group_id)?
            .ok_or_else(|| anyhow!("No conversation found for group ID {:?}", group_id))?;
        let conversation_id = conversation.id();

        let mut group = Group::load(&connection, group_id)?
            .ok_or_else(|| anyhow!("No group found for group ID {:?}", group_id))?;
        drop(connection);

        // MLSMessage Phase 2: Process the message
        let (processed_message, we_were_removed, sender_client_id) = group
            .process_message(
                self.inner.connection.clone(),
                &self.inner.api_clients,
                protocol_message,
            )
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
        let mut connection = self.inner.connection.lock().await;
        let mut transaction = connection.transaction()?;
        group.store_update(&transaction)?;

        let conversation_messages =
            self.store_messages(&mut transaction, conversation_id, group_messages)?;
        transaction.commit()?;
        Ok(match (conversation_messages, conversation_changed) {
            (messages, true) => {
                ProcessQsMessageResult::ConversationChanged(conversation_id, messages)
            }
            (messages, false) => ProcessQsMessageResult::ConversationMessages(messages),
        })
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
        )?];
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
        let connection = self.inner.connection.lock().await;
        group.store_proposal(&connection, proposal)?;
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
        let connection = self.inner.connection.lock().await;
        let mut conversation =
            Conversation::load(&connection, &conversation_id)?.ok_or(anyhow!(
                "Can't find conversation with id {}",
                conversation_id.as_uuid()
            ))?;
        drop(connection);
        let mut conversation_changed = false;

        let mut notifier = self.store_notifier();

        if let ConversationType::UnconfirmedConnection(ref user_name) =
            conversation.conversation_type()
        {
            let user_name = user_name.clone();
            // Check if it was an external commit and if the user name matches
            if !matches!(sender, Sender::NewMemberCommit)
                && sender_client_id.user_name() == user_name
            {
                // TODO: Handle the fact that an unexpected user joined the connection group.
            }
            // UnconfirmedConnection Phase 1: Load up the partial contact and decrypt the
            // friendship package
            let connection = self.inner.connection.lock().await;
            let partial_contact = PartialContact::load(&connection, &user_name)?.ok_or(anyhow!(
                "No partial contact found for user name {}",
                user_name
            ))?;
            drop(connection);

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

            // UnconfirmedConnection Phase 2: Store the user profile of the sender and the contact.
            let mut connection = self.inner.connection.lock().await;
            friendship_package
                .user_profile
                .update(&connection, &mut notifier)?;

            // Set the picture of the conversation to the one of the contact.
            let conversation_picture_option = friendship_package
                .user_profile
                .profile_picture()
                .map(|asset| match asset {
                    Asset::Value(value) => value.to_owned(),
                });

            conversation.set_conversation_picture(
                &connection,
                &mut notifier,
                conversation_picture_option,
            )?;
            let mut transaction = connection.transaction()?;
            // Now we can turn the partial contact into a full one.
            partial_contact.mark_as_complete(
                &mut transaction,
                &mut notifier,
                friendship_package,
                sender_client_id.clone(),
            )?;
            transaction.commit()?;

            conversation.confirm(&connection, &mut notifier)?;
            conversation_changed = true;
        }

        // StagedCommitMessage Phase 2: Merge the staged commit into the group.

        // If we were removed, we set the group to inactive.
        let connection = self.inner.connection.lock().await;
        if we_were_removed {
            let past_members = group.members(&connection).into_iter().collect();
            conversation.set_inactive(&connection, &mut notifier, past_members)?;
        }
        let group_messages =
            group.merge_pending_commit(&connection, staged_commit, ds_timestamp)?;

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
