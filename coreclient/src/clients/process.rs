// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{bail, Result};
use phnxtypes::{
    crypto::{ear::EarDecryptable, hpke::HpkeDecryptable},
    identifiers::QualifiedGroupId,
    messages::{
        client_as::ExtractedAsQueueMessagePayload,
        client_ds::{
            ExtractedQsQueueMessage, ExtractedQsQueueMessagePayload, InfraAadMessage,
            InfraAadPayload, JoinConnectionGroupParamsAad,
        },
        client_ds_out::ExternalCommitInfoIn,
        QueueMessage,
    },
};
use tls_codec::DeserializeBytes;

use crate::conversations::ConversationType;

use self::user_profiles::Asset;

use super::{connection_establishment::ConnectionEstablishmentPackageIn, *};

pub enum ProcessQsMessageResult {
    NewConversation(ConversationId),
    ConversationChanged(ConversationId, Vec<ConversationMessage>),
    ConversationMessages(Vec<ConversationMessage>),
}

impl SelfUser {
    /// Decrypt a `QueueMessage` received from the QS queue.
    pub fn decrypt_qs_queue_message(
        &self,
        qs_message_ciphertext: QueueMessage,
    ) -> Result<ExtractedQsQueueMessage> {
        let queue_ratchet_store = self.queue_ratchet_store();
        let mut qs_queue_ratchet = queue_ratchet_store.get_qs_queue_ratchet()?;
        let payload = qs_queue_ratchet.decrypt(qs_message_ciphertext)?;
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
    pub async fn process_qs_message(
        &mut self,
        qs_queue_message: ExtractedQsQueueMessage,
    ) -> Result<ProcessQsMessageResult> {
        // TODO: We should verify whether the messages are valid infra messages, i.e.
        // if it doesn't mix requests, etc. I think the DS already does some of this
        // and we might be able to re-use code.

        // Keep track of freshly joined groups s.t. we can later update our user auth keys.
        let ds_timestamp = qs_queue_message.timestamp;
        let processing_result = match qs_queue_message.payload {
            ExtractedQsQueueMessagePayload::WelcomeBundle(welcome_bundle) => {
                let group_store = self.group_store();
                let group = group_store
                    .join_group(
                        &self.crypto_backend(),
                        welcome_bundle,
                        &self.key_store.wai_ear_key,
                        self.leaf_key_store(),
                        self.as_credential_store(),
                    )
                    .await?;
                let group_id = group.group_id().clone();

                // Store the user profiles of the group members if they don't
                // exist yet.
                group
                    .members(&self.sqlite_connection)
                    .into_iter()
                    .try_for_each(|user_name| {
                        UserProfile::new(user_name, None, None).store(&self.sqlite_connection)
                    })?;

                // Set the conversation attributes according to the group's
                // group data.
                let group_data = group.group_data().ok_or(anyhow!("No group data"))?;
                let attributes: ConversationAttributes =
                    serde_json::from_slice(group_data.bytes())?;

                let conversation =
                    Conversation::new_group_conversation(group_id.clone(), attributes);
                // If we've been in that conversation before, we delete the old
                // conversation first and then create a new one. We do leave the
                // messages intact, though.
                let transaction = self.sqlite_connection.transaction()?;
                Conversation::delete(&transaction, conversation.id())?;
                conversation.store(&transaction)?;
                transaction.commit()?;

                ProcessQsMessageResult::NewConversation(conversation.id())
            }
            ExtractedQsQueueMessagePayload::MlsMessage(mls_message) => {
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
                let group_id = protocol_message.group_id();
                let conversation =
                    Conversation::load_by_group_id(&self.sqlite_connection, group_id)?
                        .ok_or(anyhow!("No conversation found for group ID {:?}", group_id))?;
                let conversation_id = conversation.id();

                let group_store = self.group_store();
                let mut group = group_store
                    .get(group_id)?
                    .ok_or(anyhow!("No group found for group ID {:?}", group_id))?;
                let as_credential_store = self.as_credential_store();
                let (processed_message, we_were_removed, sender_client_id) = group
                    .process_message(
                        &self.crypto_backend(),
                        protocol_message,
                        &as_credential_store,
                    )
                    .await?;

                let sender = processed_message.sender().clone();
                let aad = processed_message.authenticated_data().to_vec();
                // `conversation_changed` indicates whether the state of the conversation was updated
                let (group_messages, conversation_changed) = match processed_message.into_content()
                {
                    ProcessedMessageContent::ApplicationMessage(application_message) => {
                        let group_messages = vec![TimestampedMessage::from_application_message(
                            application_message,
                            ds_timestamp,
                            sender_client_id.user_name(),
                        )?];
                        (group_messages, false)
                    }
                    ProcessedMessageContent::ProposalMessage(proposal) => {
                        // For now, we don't to anything here. The proposal
                        // was processed by the MLS group and will be
                        // committed with the next commit.
                        group.store_proposal(*proposal)?;
                        (vec![], false)
                    }
                    ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                        // If a client joined externally, we check if the
                        // group belongs to an unconfirmed conversation.
                        let mut conversation =
                            Conversation::load(&self.sqlite_connection, &conversation_id)?.ok_or(
                                anyhow!(
                                    "Can't find conversation with id {}",
                                    conversation_id.as_uuid()
                                ),
                            )?;
                        let mut conversation_changed = false;

                        if let ConversationType::UnconfirmedConnection(ref user_name) =
                            conversation.conversation_type()
                        {
                            let user_name = user_name.clone().into();
                            // Check if it was an external commit and if the user name matches
                            if !matches!(sender, Sender::NewMemberCommit)
                                && sender_client_id.user_name() == user_name
                            {
                                // TODO: Handle the fact that an unexpected user joined the connection group.
                            }
                            // Load up the partial contact and decrypt the
                            // friendship package
                            let partial_contact =
                                PartialContact::load(&self.sqlite_connection, &user_name)?.ok_or(
                                    anyhow!("No partial contact found for user name {}", user_name),
                                )?;

                            // This is a bit annoying, since we already
                            // de-serialized this in the group processing
                            // function, but we need the encrypted
                            // friendship package here.
                            let encrypted_friendship_package =
                                if let InfraAadPayload::JoinConnectionGroup(payload) =
                                    InfraAadMessage::tls_deserialize_exact_bytes(&aad)?
                                        .into_payload()
                                {
                                    payload.encrypted_friendship_package
                                } else {
                                    bail!("Unexpected AAD payload")
                                };

                            let friendship_package = FriendshipPackage::decrypt(
                                &partial_contact.friendship_package_ear_key,
                                &encrypted_friendship_package,
                            )?;
                            // We also need to get the add infos
                            let mut add_infos = vec![];
                            for _ in 0..5 {
                                let key_package_batch_response = self
                                    .api_clients
                                    .get(&user_name.domain())?
                                    .qs_key_package_batch(
                                        friendship_package.friendship_token.clone(),
                                        friendship_package.add_package_ear_key.clone(),
                                    )
                                    .await?;
                                let key_packages: Vec<(KeyPackage, SignatureEarKey)> =
                                    key_package_batch_response
                                        .add_packages
                                        .into_iter()
                                        .map(|add_package| {
                                            let verified_add_package = add_package.validate(
                                                self.crypto_backend().crypto(),
                                                ProtocolVersion::default(),
                                            )?;
                                            let key_package =
                                                verified_add_package.key_package().clone();
                                            let sek = SignatureEarKey::decrypt(
                                                &friendship_package.signature_ear_key_wrapper_key,
                                                verified_add_package.encrypted_signature_ear_key(),
                                            )?;
                                            Ok((key_package, sek))
                                        })
                                        .collect::<Result<Vec<_>>>()?;
                                let qs_verifying_key_store = self.qs_verifying_key_store();
                                let qs_verifying_key =
                                    qs_verifying_key_store.get(&user_name.domain()).await?;
                                let key_package_batch = key_package_batch_response
                                    .key_package_batch
                                    .verify(qs_verifying_key.deref().deref())?;
                                let add_info = ContactAddInfos {
                                    key_package_batch,
                                    key_packages,
                                };
                                add_infos.push(add_info);
                            }

                            // Update the user profile of the sender.
                            friendship_package
                                .user_profile
                                .update(&self.sqlite_connection)?;

                            // Set the picture of the conversation to the one of the contact.
                            let conversation_picture_option = friendship_package
                                .user_profile
                                .profile_picture()
                                .map(|asset| match asset {
                                    Asset::Value(value) => value.to_owned(),
                                });

                            conversation.set_conversation_picture(
                                &self.sqlite_connection,
                                conversation_picture_option,
                            )?;
                            // Now we can turn the partial contact into a full one.
                            partial_contact.mark_as_complete(
                                &self.sqlite_connection,
                                friendship_package,
                                sender_client_id.clone(),
                            )?;

                            conversation.confirm(&self.sqlite_connection)?;
                            conversation_changed = true;
                        }
                        // If we were removed, we set the group to inactive.
                        if we_were_removed {
                            let past_members =
                                group.members(&self.sqlite_connection).into_iter().collect();
                            conversation.set_inactive(&self.sqlite_connection, past_members)?;
                        }
                        let group_messages = group.merge_pending_commit(
                            &self.crypto_backend(),
                            *staged_commit,
                            ds_timestamp,
                        )?;
                        (group_messages, conversation_changed)
                    }
                    ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                        unimplemented!()
                    }
                };
                let conversation_messages =
                    self.store_group_messages(conversation_id, group_messages)?;
                match (conversation_messages, conversation_changed) {
                    (messages, true) => {
                        ProcessQsMessageResult::ConversationChanged(conversation_id, messages)
                    }
                    (messages, false) => ProcessQsMessageResult::ConversationMessages(messages),
                }
            }
        };

        Ok(processing_result)
    }

    /// Decrypt a `QueueMessage` received from the AS queue.
    pub fn decrypt_as_queue_message(
        &self,
        as_message_ciphertext: QueueMessage,
    ) -> Result<ExtractedAsQueueMessagePayload> {
        let queue_ratchet_store = self.queue_ratchet_store();
        let mut as_queue_ratchet = queue_ratchet_store.get_as_queue_ratchet()?;
        let payload = as_queue_ratchet.decrypt(as_message_ciphertext)?;
        Ok(payload.extract()?)
    }

    /// Process a decrypted message received from the AS queue.
    ///
    /// Returns the [`ConversationId`] of any newly created conversations.
    pub async fn process_as_message(
        &mut self,
        as_message_plaintext: ExtractedAsQueueMessagePayload,
    ) -> Result<ConversationId> {
        let conversation_id = match as_message_plaintext {
            ExtractedAsQueueMessagePayload::EncryptedConnectionEstablishmentPackage(ecep) => {
                let cep_in = ConnectionEstablishmentPackageIn::decrypt(
                    ecep,
                    &self.key_store.connection_decryption_key,
                    &[],
                    &[],
                )?;
                // Fetch authentication AS credentials of the sender if we
                // don't have them already.
                let sender_domain = cep_in.sender_credential().domain();

                let as_credential_store = self.as_credential_store();
                let as_intermediate_credential = as_credential_store
                    .get(
                        &sender_domain,
                        cep_in.sender_credential().signer_fingerprint(),
                    )
                    .await?;
                let cep_tbs = cep_in.verify(as_intermediate_credential.verifying_key());
                // We create a new group and signal that fact to the user,
                // so the user can decide if they want to accept the
                // connection.

                let signature_ear_key = SignatureEarKey::random()?;
                let leaf_signer = InfraCredentialSigningKey::generate(
                    &self.key_store.signing_key,
                    &signature_ear_key,
                );
                let esek = signature_ear_key
                    .encrypt(&cep_tbs.connection_group_signature_ear_key_wrapper_key)?;

                let own_user_profile = self.own_user_profile()?;

                let encrypted_friendship_package = FriendshipPackage {
                    friendship_token: self.key_store.friendship_token.clone(),
                    add_package_ear_key: self.key_store.add_package_ear_key.clone(),
                    client_credential_ear_key: self.key_store.client_credential_ear_key.clone(),
                    signature_ear_key_wrapper_key: self
                        .key_store
                        .signature_ear_key_wrapper_key
                        .clone(),
                    wai_ear_key: self.key_store.wai_ear_key.clone(),
                    user_profile: own_user_profile,
                }
                .encrypt(&cep_tbs.friendship_package_ear_key)?;
                let ecc = self
                    .key_store
                    .signing_key
                    .credential()
                    .encrypt(&cep_tbs.connection_group_credential_key)?;

                let aad = InfraAadPayload::JoinConnectionGroup(JoinConnectionGroupParamsAad {
                    encrypted_client_information: (ecc, esek),
                    encrypted_friendship_package,
                })
                .into();

                // Fetch external commit information.
                let qgid = QualifiedGroupId::tls_deserialize_exact_bytes(
                    cep_tbs.connection_group_id.as_slice(),
                )?;
                let eci: ExternalCommitInfoIn = self
                    .api_clients
                    .get(&qgid.owning_domain)?
                    .ds_connection_group_info(
                        cep_tbs.connection_group_id.clone(),
                        &cep_tbs.connection_group_ear_key,
                    )
                    .await?;
                let group_store = self.group_store();
                let (group, commit, group_info) = group_store
                    .join_group_externally(
                        &self.crypto_backend(),
                        eci,
                        leaf_signer,
                        signature_ear_key,
                        cep_tbs.connection_group_ear_key.clone(),
                        cep_tbs.connection_group_signature_ear_key_wrapper_key,
                        cep_tbs.connection_group_credential_key,
                        &self.as_credential_store(),
                        aad,
                        self.key_store.signing_key.credential(),
                    )
                    .await?;
                let sender_client_id = cep_tbs.sender_client_credential.identity();
                let conversation_picture_option = cep_tbs
                    .friendship_package
                    .user_profile
                    .profile_picture()
                    .map(|asset| match asset {
                        Asset::Value(value) => value.to_owned(),
                    });
                let mut conversation = Conversation::new_connection_conversation(
                    group.group_id().clone(),
                    sender_client_id.user_name().clone(),
                    ConversationAttributes::new(
                        sender_client_id.user_name().to_string(),
                        conversation_picture_option,
                    ),
                )?;
                conversation.store(&self.sqlite_connection)?;
                // Store the user profile of the sender.
                cep_tbs
                    .friendship_package
                    .user_profile
                    .store(&self.sqlite_connection)?;
                // TODO: For now, we automatically confirm conversations.
                conversation.confirm(&self.sqlite_connection)?;
                // TODO: Here, we want to store a contact
                Contact::from_friendship_package(
                    sender_client_id,
                    conversation.id(),
                    cep_tbs.friendship_package,
                )
                .store(&self.sqlite_connection)?;

                let qs_client_reference = self.create_own_client_reference();

                // Send the confirmation by way of commit and group info to the DS.
                self.api_clients
                    .get(&qgid.owning_domain)?
                    .ds_join_connection_group(
                        commit,
                        group_info,
                        qs_client_reference,
                        group.user_auth_key().ok_or(anyhow!("No user auth key"))?,
                        &cep_tbs.connection_group_ear_key,
                    )
                    .await?;
                conversation.id()
            }
        };
        Ok(conversation_id)
    }

    pub fn conversation(&self, conversation_id: ConversationId) -> Option<Conversation> {
        Conversation::load(&self.sqlite_connection, &conversation_id)
            .ok()
            .flatten()
    }

    /// Get the most recent `number_of_messages` messages from the conversation
    /// with the given [`ConversationId`].
    pub fn get_messages(
        &self,
        conversation_id: ConversationId,
        number_of_messages: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let messages = ConversationMessage::load_multiple(
            &self.sqlite_connection,
            conversation_id,
            number_of_messages as u32,
        )?;
        Ok(messages)
    }

    /// Convenience function that takes a list of `QueueMessage`s retrieved from
    /// the QS, decrypts them, and processes them.
    pub async fn fully_process_qs_messages(
        &mut self,
        qs_messages: Vec<QueueMessage>,
    ) -> Result<Vec<ConversationMessage>> {
        let mut collected_conversation_messages = vec![];
        let mut new_conversations = vec![];
        for qs_message in qs_messages {
            let qs_message_plaintext = self.decrypt_qs_queue_message(qs_message)?;
            match self.process_qs_message(qs_message_plaintext).await? {
                ProcessQsMessageResult::ConversationMessages(conversation_messages) => {
                    collected_conversation_messages.extend(conversation_messages);
                }
                ProcessQsMessageResult::ConversationChanged(
                    _conversation_id,
                    conversation_messages,
                ) => {
                    collected_conversation_messages.extend(conversation_messages);
                }
                ProcessQsMessageResult::NewConversation(conversation_id) => {
                    new_conversations.push(conversation_id)
                }
            };
        }

        for conversation_id in new_conversations {
            // Update user auth keys of newly created conversations.
            self.update_user_key(conversation_id).await?;
        }

        Ok(collected_conversation_messages)
    }

    /// Convenience function that takes a list of `QueueMessage`s retrieved from
    /// the AS, decrypts them, and processes them.
    pub async fn fully_process_as_messages(
        &mut self,
        as_messages: Vec<QueueMessage>,
    ) -> Result<Vec<ConversationId>> {
        let mut conversation_ids = vec![];
        for as_message in as_messages {
            let as_message_plaintext = self.decrypt_as_queue_message(as_message)?;
            let conversation_id = self.process_as_message(as_message_plaintext).await?;
            conversation_ids.push(conversation_id);
        }
        Ok(conversation_ids)
    }
}
