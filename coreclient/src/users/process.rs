// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::messages::client_ds::ExtractedQueueMessagePayload;
use phnxbackend::messages::QueueMessage;

use super::*;

impl SelfUser {
    /// Process received messages by group. This function is meant to be called
    /// with messages received from the QS queue.
    pub fn process_messages<T: Notifiable>(
        &mut self,
        message_ciphertexts: Vec<QueueMessage>,
        notification_hub: &mut NotificationHub<T>,
    ) -> Result<(), CorelibError> {
        // Decrypt received message.
        let messages: Vec<ExtractedQueueMessagePayload> = message_ciphertexts
            .into_iter()
            .map(|message_ciphertext| {
                self.key_store
                    .qs_ratchet_key
                    .decrypt(message_ciphertext)
                    .unwrap()
                    .extract()
                    .unwrap()
            })
            .collect();

        // TODO: We should verify whether the messages are valid infra messages, i.e.
        // if it doesn't mix requests, etc. I think the DS already does some of this
        // and we might be able to re-use code.

        let mut notification_messages = vec![];
        for message in messages {
            match message {
                ExtractedQueueMessagePayload::WelcomeBundle(welcome_bundle) => {
                    let group_id = self.group_store.join_group(
                        &self.crypto_backend,
                        welcome_bundle,
                        &self.key_store.wai_ear_key,
                        &mut self.key_store.leaf_signers,
                        &self.key_store.as_intermediate_credentials,
                        &self.contacts,
                    );

                    let conversation_id = Uuid::new_v4();
                    let attributes = ConversationAttributes {
                        title: "New conversation".to_string(),
                    };
                    self.conversation_store.create_group_conversation(
                        conversation_id,
                        group_id,
                        attributes,
                    );
                    notification_hub.dispatch_conversation_notification();
                }
                ExtractedQueueMessagePayload::MlsMessage(mls_message) => {
                    let protocol_message: ProtocolMessage = match mls_message.extract() {
                        MlsMessageInBody::PublicMessage(handshake_message) => handshake_message.into(),
                        // Only application messages are private
                        MlsMessageInBody::PrivateMessage(app_msg) => app_msg.into(),
                        // Welcomes always come as a WelcomeBundle, not as an MLSMessage.
                        MlsMessageInBody::Welcome(_) |
                        // Neither GroupInfos nor KeyPackages should come from the queue.
                        MlsMessageInBody::GroupInfo(_) | MlsMessageInBody::KeyPackage(_) => return Err(CorelibError::NetworkError),
                    };
                    let group_id = protocol_message.group_id();
                    let conversation_id = self
                        .conversation_store
                        .conversation_by_group_id(group_id)
                        .unwrap()
                        .id;
                    match self.group_store.get_group_mut(group_id) {
                        Some(group) => {
                            let processed_message = group.process_message(
                                &self.crypto_backend,
                                protocol_message,
                                &self.key_store.as_intermediate_credentials,
                            );
                            match processed_message {
                                Ok(processed_message) => {
                                    let sender_credential = processed_message.credential().clone();
                                    let conversation_messages = match processed_message
                                        .into_content()
                                    {
                                        ProcessedMessageContent::ApplicationMessage(
                                            application_message,
                                        ) => application_message_to_conversation_messages(
                                            &sender_credential,
                                            application_message,
                                        ),
                                        ProcessedMessageContent::ProposalMessage(_) => {
                                            unimplemented!()
                                        }
                                        ProcessedMessageContent::StagedCommitMessage(
                                            staged_commit,
                                        ) => staged_commit_to_conversation_messages(
                                            &sender_credential.identity().to_vec().into(),
                                            &staged_commit,
                                        ),
                                        ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                                            unimplemented!()
                                        }
                                    };

                                    for conversation_message in conversation_messages {
                                        let dispatched_conversation_message =
                                            DispatchedConversationMessage {
                                                conversation_id,
                                                conversation_message: conversation_message.clone(),
                                            };
                                        self.conversation_store.store_message(
                                            &conversation_id,
                                            conversation_message,
                                        )?;
                                        notification_messages.push(dispatched_conversation_message);
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "Error occured while processing inbound messages: {:?}",
                                        e
                                    );
                                }
                            }
                        }
                        None => {
                            return Err(CorelibError::GroupStore(GroupStoreError::UnknownGroup))
                        }
                    }
                }
            }
        }
        // TODO: We notify in bulk here. We might want to change this in the future.
        for notification_message in notification_messages.clone() {
            notification_hub.dispatch_message_notification(notification_message);
        }
        Ok(())
    }

    /// Get existing conversations
    pub fn get_conversations(&self) -> Vec<Conversation> {
        self.conversation_store.conversations()
    }

    pub fn get_messages(&self, conversation_id: &Uuid, last_n: usize) -> Vec<ConversationMessage> {
        self.conversation_store.messages(conversation_id, last_n)
    }
}
