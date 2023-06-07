// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::crypto::ear::EarDecryptable;
use phnxbackend::messages::{client_ds::QueueMessagePayload, QueueMessage};

use super::*;

impl SelfUser {
    /// Process received messages by group. This function is meant to be called
    /// with messages received from the QS queue.
    pub fn process_messages(
        &mut self,
        message_ciphertexts: Vec<QueueMessage>,
    ) -> Result<Vec<DispatchedConversationMessage>, CorelibError> {
        // Decrypt received message.
        let messages = message_ciphertexts
            .into_iter()
            .map(|message_ciphertext| {
                let serialized_message =
                    self.key_store.qs_ratchet_key.decrypt(message_ciphertext)?;
                let message =
                    MlsMessageIn::tls_deserialize(&mut serialized_message.payload).unwrap();
                Ok(message)
            })
            .collect::<Result<Vec<_>, CorelibError>>()?;

        // TODO: We should verify whether the messages are valid infra messages, i.e.
        // if it doesn't mix requests, etc. I think the DS already does some of this
        // and we might be able to re-use code.

        let mut notification_messages = vec![];
        for message in messages {
            // TODO: Check version?
            match message.extract() {
                MlsMessageInBody::PublicMessage(handshake_message) => todo!(),
                // Only application messages are private
                MlsMessageInBody::PrivateMessage(app_msg) => todo!(),
                MlsMessageInBody::Welcome(welcome) => todo!(),
                // Neither GroupInfos nor KeyPackages should come from the queue.
                MlsMessageInBody::GroupInfo(_) | MlsMessageInBody::KeyPackage(_) => todo!(),
            }
        }

        match self.group_store.get_group_mut(&group_id) {
            Some(group) => {
                for message in messages {
                    let processed_message = group.process_message(&self.crypto_backend, message);
                    match processed_message {
                        Ok(processed_message) => {
                            let sender_credential = processed_message.credential().clone();
                            let conversation_messages = match processed_message.into_content() {
                                ProcessedMessageContent::ApplicationMessage(
                                    application_message,
                                ) => application_message_to_conversation_messages(
                                    &sender_credential,
                                    application_message,
                                ),
                                ProcessedMessageContent::ProposalMessage(_) => {
                                    unimplemented!()
                                }
                                ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                                    staged_commit_to_conversation_messages(
                                        &sender_credential,
                                        &staged_commit,
                                    )
                                }
                                ProcessedMessageContent::ExternalJoinProposalMessage(_) => todo!(),
                            };

                            for conversation_message in conversation_messages {
                                let dispatched_conversation_message =
                                    DispatchedConversationMessage {
                                        conversation_id: UuidBytes::from_uuid(&group_id),
                                        conversation_message: conversation_message.clone(),
                                    };
                                self.conversation_store
                                    .store_message(&group_id, conversation_message)?;
                                notification_messages.push(dispatched_conversation_message);
                            }
                        }
                        Err(e) => {
                            println!("Error occured while processing inbound messages: {:?}", e);
                        }
                    }
                }

                Ok(notification_messages)
            }
            None => Err(CorelibError::GroupStore(GroupStoreError::UnknownGroup)),
        }
    }
}
