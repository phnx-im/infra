// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, Result, bail, ensure};
use mimi_content::{
    Disposition, MessageStatus, MessageStatusReport, MimiContent, NestedPartContent,
};
use openmls::{
    group::QueuedProposal,
    prelude::{
        ApplicationMessage, MlsMessageBodyIn, MlsMessageIn, ProcessedMessageContent,
        ProtocolMessage, Sender,
    },
};
use phnxcommon::{
    codec::PhnxCodec,
    credentials::ClientCredential,
    crypto::{ear::EarDecryptable, indexed_aead::keys::UserProfileKey},
    identifiers::{MimiId, QualifiedGroupId, UserHandle, UserId},
    messages::{
        QueueMessage,
        client_ds::{
            ExtractedQsQueueMessage, ExtractedQsQueueMessagePayload, InfraAadMessage,
            InfraAadPayload, UserProfileKeyUpdateParams, WelcomeBundle,
        },
    },
    time::TimeStamp,
};
use sqlx::{Acquire, SqliteTransaction};
use tls_codec::DeserializeBytes;
use tracing::error;

use crate::{
    ContentMessage, ConversationMessage, Message,
    contacts::HandleContact,
    conversations::{ConversationType, StatusRecord, messages::edit::MessageEdit},
    groups::{Group, client_auth_info::StorableClientCredential, process::ProcessMessageResult},
    key_stores::indexed_keys::StorableIndexedKey,
    store::StoreNotifier,
};

use super::{
    Conversation, ConversationAttributes, ConversationId, CoreUser, FriendshipPackage,
    TimestampedMessage, anyhow,
};
use crate::key_stores::queue_ratchets::StorableQsQueueRatchet;

pub enum ProcessQsMessageResult {
    None,
    NewConversation(ConversationId),
    ConversationChanged(ConversationId, Vec<ConversationMessage>),
    ConversationMessages(Vec<ConversationMessage>),
}

#[derive(Debug)]
pub struct ProcessedQsMessages {
    pub new_conversations: Vec<ConversationId>,
    pub changed_conversations: Vec<ConversationId>,
    pub new_messages: Vec<ConversationMessage>,
    pub errors: Vec<anyhow::Error>,
}

#[derive(Default)]
struct ApplicationMessagesHandlerResult {
    new_messages: Vec<TimestampedMessage>,
    updated_messages: Vec<ConversationMessage>,
    conversation_changed: bool,
}

impl CoreUser {
    /// Decrypt a `QueueMessage` received from the QS queue.
    pub async fn decrypt_qs_queue_message(
        &self,
        qs_message_ciphertext: QueueMessage,
    ) -> Result<ExtractedQsQueueMessage> {
        self.with_transaction(async |txn| {
            let mut qs_queue_ratchet = StorableQsQueueRatchet::load(txn.as_mut()).await?;
            let payload = qs_queue_ratchet.decrypt(qs_message_ciphertext)?;
            qs_queue_ratchet.update_ratchet(txn.as_mut()).await?;
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
            ExtractedQsQueueMessagePayload::UserProfileKeyUpdate(
                user_profile_key_update_params,
            ) => {
                self.handle_user_profile_key_update(user_profile_key_update_params)
                    .await
            }
        }
    }

    async fn handle_welcome_bundle(
        &self,
        welcome_bundle: WelcomeBundle,
    ) -> Result<ProcessQsMessageResult> {
        // WelcomeBundle Phase 1: Join the group. This might involve
        // loading AS credentials or fetching them from the AS.
        let (own_profile_key, own_profile_key_in_group, group, conversation_id) = self
            .with_transaction_and_notifier(async |txn, notifier| {
                let (group, member_profile_info) = Group::join_group(
                    welcome_bundle,
                    &self.inner.key_store.wai_ear_key,
                    txn,
                    &self.inner.api_clients,
                    self.signing_key(),
                )
                .await?;
                let group_id = group.group_id().clone();

                // WelcomeBundle Phase 2: Fetch the user profiles of the group members
                // and decrypt them.

                // TODO: This can fail in some cases. If it does, we should fetch and
                // process messages and then try again.
                let mut own_profile_key_in_group = None;
                for profile_info in member_profile_info {
                    // TODO: Don't fetch while holding a transaction!
                    if profile_info.client_credential.identity() == self.user_id() {
                        // We already have our own profile info.
                        own_profile_key_in_group = Some(profile_info.user_profile_key);
                        continue;
                    }
                    self.fetch_and_store_user_profile(txn, notifier, profile_info)
                        .await?;
                }

                let Some(own_profile_key_in_group) = own_profile_key_in_group else {
                    bail!("No profile info for our user found");
                };

                // WelcomeBundle Phase 3: Store the user profiles of the group
                // members if they don't exist yet and store the group and the
                // new conversation.

                // Set the conversation attributes according to the group's
                // group data.
                let group_data = group.group_data().context("No group data")?;
                let attributes: ConversationAttributes = PhnxCodec::from_slice(group_data.bytes())?;

                let conversation =
                    Conversation::new_group_conversation(group_id.clone(), attributes);
                let own_profile_key = UserProfileKey::load_own(txn.as_mut()).await?;
                // If we've been in that conversation before, we delete the old
                // conversation (and the corresponding MLS group) first and then
                // create a new one. We do leave the messages intact, though.
                Conversation::delete(txn.as_mut(), notifier, conversation.id()).await?;
                Group::delete_from_db(txn, &group_id).await?;
                group.store(txn.as_mut()).await?;
                conversation.store(txn.as_mut(), notifier).await?;

                Ok((
                    own_profile_key,
                    own_profile_key_in_group,
                    group,
                    conversation.id(),
                ))
            })
            .await?;

        // WelcomeBundle Phase 4: Check whether our user profile key is up to
        // date and if not, update it.
        if own_profile_key_in_group != own_profile_key {
            let qualified_group_id = QualifiedGroupId::try_from(group.group_id().clone())?;
            let api_client = self
                .inner
                .api_clients
                .get(qualified_group_id.owning_domain())?;
            let encrypted_profile_key =
                own_profile_key.encrypt(group.identity_link_wrapper_key(), self.user_id())?;
            let params = UserProfileKeyUpdateParams {
                group_id: group.group_id().clone(),
                sender_index: group.own_index(),
                user_profile_key: encrypted_profile_key,
            };
            api_client
                .ds_user_profile_key_update(params, self.signing_key(), group.group_state_ear_key())
                .await?;
        }

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
        let group_id = protocol_message.group_id().clone();

        let (conversation_messages, conversation_changed, conversation_id, profile_infos) = self
            .with_transaction_and_notifier(async |txn, notifier| {
                let conversation = Conversation::load_by_group_id(txn.as_mut(), &group_id)
                    .await?
                    .ok_or_else(|| anyhow!("No conversation found for group ID {:?}", group_id))?;
                let conversation_id = conversation.id();

                let mut group = Group::load_clean(txn, &group_id)
                    .await?
                    .ok_or_else(|| anyhow!("No group found for group ID {:?}", group_id))?;

                // MLSMessage Phase 2: Process the message
                let ProcessMessageResult {
                    processed_message,
                    we_were_removed,
                    sender_client_credential,
                    profile_infos,
                } = group
                    .process_message(txn, &self.inner.api_clients, protocol_message)
                    .await?;

                let sender = processed_message.sender().clone();
                let aad = processed_message.aad().to_vec();

                // `conversation_changed` indicates whether the state of the conversation was updated
                let (new_messages, updated_messages, conversation_changed) =
                    match processed_message.into_content() {
                        ProcessedMessageContent::ApplicationMessage(application_message) => {
                            let ApplicationMessagesHandlerResult {
                                new_messages,
                                updated_messages,
                                conversation_changed,
                            } = self
                                .handle_application_message(
                                    txn,
                                    notifier,
                                    &group,
                                    application_message,
                                    ds_timestamp,
                                    sender_client_credential.identity(),
                                )
                                .await?;
                            (new_messages, updated_messages, conversation_changed)
                        }
                        ProcessedMessageContent::ProposalMessage(proposal) => {
                            let (new_messages, updated) = self
                                .handle_proposal_message(txn, &mut group, *proposal)
                                .await?;
                            (new_messages, Vec::new(), updated)
                        }
                        ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                            let (new_messages, updated) = self
                                .handle_staged_commit_message(
                                    txn,
                                    &mut group,
                                    conversation,
                                    *staged_commit,
                                    aad,
                                    ds_timestamp,
                                    &sender,
                                    &sender_client_credential,
                                    we_were_removed,
                                )
                                .await?;
                            (new_messages, Vec::new(), updated)
                        }
                        ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                            let (new_messages, updated) =
                                self.handle_external_join_proposal_message()?;
                            (new_messages, Vec::new(), updated)
                        }
                    };

                // MLSMessage Phase 3: Store the updated group and the messages.
                group.store_update(txn.as_mut()).await?;

                let mut conversation_messages =
                    Self::store_new_messages(txn, notifier, conversation_id, new_messages).await?;
                for updated_message in updated_messages {
                    updated_message.update(txn.as_mut(), notifier).await?;
                    conversation_messages.push(updated_message);
                }

                Ok((
                    conversation_messages,
                    conversation_changed,
                    conversation_id,
                    profile_infos,
                ))
            })
            .await?;

        // Send delivery receipts for incoming messages
        // TODO: Queue this and run the network requests batched together in a background task

        let delivered_receipts = conversation_messages.iter().filter_map(|message| {
            if let Message::Content(content_message) = message.message()
                && let Disposition::Render | Disposition::Attachment =
                    content_message.content().nested_part.disposition
                && let Some(mimi_id) = content_message.mimi_id()
            {
                Some((mimi_id, MessageStatus::Delivered))
            } else {
                None
            }
        });
        self.send_delivery_receipts(conversation_id, delivered_receipts)
            .await?;

        let res = match (conversation_messages, conversation_changed) {
            (messages, true) => {
                ProcessQsMessageResult::ConversationChanged(conversation_id, messages)
            }
            (messages, false) => ProcessQsMessageResult::ConversationMessages(messages),
        };

        // MLSMessage Phase 4: Fetch user profiles of new clients and store them.
        self.with_transaction_and_notifier(async |txn, notifier| {
            for client in profile_infos {
                self.fetch_and_store_user_profile(&mut *txn, notifier, client)
                    .await?;
            }
            Ok(())
        })
        .await?;

        Ok(res)
    }

    /// Returns a conversation message if it should be stored, otherwise an empty vec.
    ///
    /// Also returns whether the conversation should be notified as updated.
    async fn handle_application_message(
        &self,
        txn: &mut SqliteTransaction<'_>,
        notifier: &mut StoreNotifier,
        group: &Group,
        application_message: ApplicationMessage,
        ds_timestamp: TimeStamp,
        sender: &UserId,
    ) -> anyhow::Result<ApplicationMessagesHandlerResult> {
        let mut content = MimiContent::deserialize(&application_message.into_bytes());

        // Delivery receipt
        if let Ok(content) = &content
            && let NestedPartContent::SinglePart {
                content_type,
                content: report_content,
            } = &content.nested_part.part
            && content_type == "application/mimi-message-status"
        {
            let report = MessageStatusReport::deserialize(report_content)?;
            StatusRecord::borrowed(sender, report, ds_timestamp)
                .store_report(txn, notifier)
                .await?;
            // Delivery receipt messages are not stored
            return Ok(Default::default());
        }

        // Message edit
        if let Ok(content) = &mut content
            && let Some(replaces) = content.replaces.as_ref()
            && let Ok(mimi_id) = MimiId::from_slice(replaces)
        {
            // Don't fail here, otherwise message processing of other messages will fail.
            let mut savepoint_txn = txn.begin().await?;
            let message = handle_message_edit(
                &mut savepoint_txn,
                notifier,
                group,
                ds_timestamp,
                sender,
                mimi_id,
                std::mem::take(content),
            )
            .await
            .inspect_err(|error| {
                error!(%error, "Failed to handle message edit; skipping");
            })
            .ok();
            if message.is_some() {
                savepoint_txn.commit().await?;
            }

            return Ok(ApplicationMessagesHandlerResult {
                updated_messages: message.into_iter().collect(),
                conversation_changed: true,
                ..Default::default()
            });
        }

        let message =
            TimestampedMessage::from_mimi_content_result(content, ds_timestamp, sender, group);
        Ok(ApplicationMessagesHandlerResult {
            new_messages: vec![message],
            conversation_changed: true,
            ..Default::default()
        })
    }

    async fn handle_proposal_message(
        &self,
        txn: &mut SqliteTransaction<'_>,
        group: &mut Group,
        proposal: QueuedProposal,
    ) -> anyhow::Result<(Vec<TimestampedMessage>, bool)> {
        // For now, we don't to anything here. The proposal
        // was processed by the MLS group and will be
        // committed with the next commit.
        group.store_proposal(txn.as_mut(), proposal)?;
        Ok((vec![], false))
    }

    #[expect(clippy::too_many_arguments)]
    async fn handle_staged_commit_message(
        &self,
        txn: &mut SqliteTransaction<'_>,
        group: &mut Group,
        mut conversation: Conversation,
        staged_commit: openmls::prelude::StagedCommit,
        aad: Vec<u8>,
        ds_timestamp: TimeStamp,
        sender: &openmls::prelude::Sender,
        sender_client_credential: &ClientCredential,
        we_were_removed: bool,
    ) -> anyhow::Result<(Vec<TimestampedMessage>, bool)> {
        // If a client joined externally, we check if the
        // group belongs to an unconfirmed conversation.

        // StagedCommitMessage Phase 1: Confirm the conversation if unconfirmed
        let mut notifier = self.store_notifier();

        let conversation_changed = match &conversation.conversation_type() {
            ConversationType::HandleConnection(handle) => {
                let handle = handle.clone();
                self.handle_unconfirmed_conversation(
                    txn,
                    &mut notifier,
                    aad,
                    sender,
                    sender_client_credential,
                    &mut conversation,
                    &handle,
                )
                .await?;
                true
            }
            _ => false,
        };

        // StagedCommitMessage Phase 2: Merge the staged commit into the group.

        // If we were removed, we set the group to inactive.
        if we_were_removed {
            let past_members = group.members(txn.as_mut()).await.into_iter().collect();
            conversation
                .set_inactive(txn.as_mut(), &mut notifier, past_members)
                .await?;
        }
        let group_messages = group
            .merge_pending_commit(txn, staged_commit, ds_timestamp)
            .await?;

        notifier.notify();

        Ok((group_messages, conversation_changed))
    }

    #[expect(clippy::too_many_arguments)]
    async fn handle_unconfirmed_conversation(
        &self,
        txn: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        notifier: &mut StoreNotifier,
        aad: Vec<u8>,
        sender: &Sender,
        sender_client_credential: &ClientCredential,
        conversation: &mut Conversation,
        handle: &UserHandle,
    ) -> Result<(), anyhow::Error> {
        // Check if it was an external commit
        ensure!(
            matches!(sender, Sender::NewMemberCommit),
            "Incoming commit to ConnectionGroup was not an external commit"
        );
        let user_id = sender_client_credential.identity();

        // UnconfirmedConnection Phase 1: Load up the partial contact and decrypt the
        // friendship package
        let contact = HandleContact::load(txn.as_mut(), handle)
            .await?
            .with_context(|| format!("No contact found with handle: {}", handle.plaintext()))?;

        // This is a bit annoying, since we already
        // de-serialized this in the group processing
        // function, but we need the encrypted
        // friendship package here.
        let encrypted_friendship_package = if let InfraAadPayload::JoinConnectionGroup(payload) =
            InfraAadMessage::tls_deserialize_exact_bytes(&aad)?.into_payload()
        {
            payload.encrypted_friendship_package
        } else {
            bail!("Unexpected AAD payload")
        };

        let friendship_package = FriendshipPackage::decrypt(
            &contact.friendship_package_ear_key,
            &encrypted_friendship_package,
        )?;

        let user_profile_key = UserProfileKey::from_base_secret(
            friendship_package.user_profile_base_secret.clone(),
            user_id,
        )?;

        // UnconfirmedConnection Phase 2: Fetch the user profile.
        let user_profile_key_index = user_profile_key.index().clone();
        self.fetch_and_store_user_profile(
            txn,
            notifier,
            (sender_client_credential.clone(), user_profile_key),
        )
        .await?;

        // Now we can turn the partial contact into a full one.
        let contact = contact
            .mark_as_complete(
                txn,
                notifier,
                user_id.clone(),
                friendship_package,
                user_profile_key_index,
            )
            .await?;

        conversation
            .confirm(txn.as_mut(), notifier, contact.user_id)
            .await?;

        Ok(())
    }

    async fn handle_user_profile_key_update(
        &self,
        params: UserProfileKeyUpdateParams,
    ) -> anyhow::Result<ProcessQsMessageResult> {
        let mut connection = self.pool().acquire().await?;

        // Phase 1: Load the group and the sender.
        let group = Group::load(&mut connection, &params.group_id)
            .await?
            .context("No group found")?;
        let sender = group
            .client_by_index(&mut connection, params.sender_index)
            .await
            .context("No sender found")?;
        let sender_credential =
            StorableClientCredential::load_by_user_id(&mut *connection, &sender)
                .await?
                .context("No sender credential found")?;

        // Phase 2: Decrypt the new user profile key
        let new_user_profile_key = UserProfileKey::decrypt(
            group.identity_link_wrapper_key(),
            &params.user_profile_key,
            &sender,
        )?;

        // Phase 3: Fetch and store the (new) user profile and key
        self.with_notifier(async |notifier| {
            self.fetch_and_store_user_profile(
                &mut connection,
                notifier,
                (sender_credential.into(), new_user_profile_key),
            )
            .await
        })
        .await?;

        Ok(ProcessQsMessageResult::None)
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
        let mut errors = vec![];
        for qs_message in qs_messages {
            let qs_message_plaintext = self.decrypt_qs_queue_message(qs_message).await?;
            let processed = match self.process_qs_message(qs_message_plaintext).await {
                Ok(processed) => processed,
                Err(e) => {
                    error!(error = %e, "Processing message failed");
                    errors.push(e);
                    continue;
                }
            };

            match processed {
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
                ProcessQsMessageResult::None => {}
            };
        }

        Ok(ProcessedQsMessages {
            new_conversations,
            changed_conversations,
            new_messages,
            errors,
        })
    }
}

async fn handle_message_edit(
    txn: &mut SqliteTransaction<'_>,
    notifier: &mut StoreNotifier,
    group: &Group,
    ds_timestamp: TimeStamp,
    sender: &UserId,
    replaces: MimiId,
    content: MimiContent,
) -> anyhow::Result<ConversationMessage> {
    let is_delete = content.nested_part.part == NestedPartContent::NullPart;

    // First try to directly load the original message by mimi id (non-edited message) and fallback
    // to the history of edits otherwise.
    let mut message = match ConversationMessage::load_by_mimi_id(txn.as_mut(), &replaces).await? {
        Some(message) => message,
        None => {
            let message_id = MessageEdit::find_message_id(txn.as_mut(), &replaces)
                .await?
                .with_context(|| {
                    format!("Original message id not found for editing; mimi_id = {replaces:?}")
                })?;

            ConversationMessage::load(txn.as_mut(), message_id)
                .await?
                .with_context(|| {
                    format!("Original message not found for editing; message_id = {message_id:?}")
                })?
        }
    };

    let original_mimi_id = message
        .message()
        .mimi_id()
        .context("Original message does not have mimi id")?;
    let original_sender = message
        .message()
        .sender()
        .context("Original message does not have sender")?;
    let original_mimi_content = message
        .message()
        .mimi_content()
        .context("Original message does not have mimi content")?;

    // TODO: Use mimi-room-policy for capabilities
    ensure!(
        original_sender == sender,
        "Only edits and deletes from original users are allowed for now"
    );

    if !is_delete {
        // Store message edit
        MessageEdit::new(
            original_mimi_id,
            message.id(),
            ds_timestamp,
            original_mimi_content,
        )
        .store(txn.as_mut())
        .await?;
    }

    // Update the original message
    let is_sent = true;
    message.set_content_message(ContentMessage::new(
        original_sender.clone(),
        is_sent,
        content,
        group.group_id(),
    ));
    message.set_edited_at(ds_timestamp);
    message.set_status(MessageStatus::Unread);

    // Clear the status of the message
    StatusRecord::clear(txn.as_mut(), notifier, message.id()).await?;

    Conversation::mark_as_unread(txn, notifier, message.conversation_id(), message.id()).await?;

    Ok(message)
}
