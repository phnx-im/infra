// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxbackend::crypto::ear::EarDecryptable;
use phnxbackend::crypto::hpke::HpkeDecryptable;
use phnxbackend::messages::client_as::ExtractedAsQueueMessagePayload;
use phnxbackend::messages::client_as_out::ConnectionEstablishmentPackageIn;
use phnxbackend::messages::client_ds::{
    ExtractedQsQueueMessagePayload, InfraAadMessage, InfraAadPayload, JoinConnectionGroupParamsAad,
};
use phnxbackend::messages::client_ds_out::ExternalCommitInfoIn;
use phnxbackend::messages::QueueMessage;
use phnxbackend::qs::{ClientConfig, Fqdn, QsClientReference};
use tls_codec::DeserializeBytes;

use super::*;

impl<T: Notifiable> SelfUser<T> {
    /// Process received messages by group. This function is meant to be called
    /// with messages received from the QS queue.
    pub async fn process_qs_messages(
        &mut self,
        message_ciphertexts: Vec<QueueMessage>,
    ) -> Result<(), CorelibError> {
        // Decrypt received message.
        let messages: Vec<ExtractedQsQueueMessagePayload> = message_ciphertexts
            .into_iter()
            .map(|message_ciphertext| {
                self.key_store
                    .qs_queue_ratchet
                    .decrypt(message_ciphertext)
                    .unwrap()
                    .extract()
                    .unwrap()
            })
            .collect();

        // TODO: We should verify whether the messages are valid infra messages, i.e.
        // if it doesn't mix requests, etc. I think the DS already does some of this
        // and we might be able to re-use code.

        // Keep track of freshly joined groups s.t. we can later update our user auth keys.
        let mut freshly_joined_groups = vec![];
        for message in messages {
            match message {
                ExtractedQsQueueMessagePayload::WelcomeBundle(welcome_bundle) => {
                    let group_id = self.group_store.join_group(
                        &self.crypto_backend,
                        welcome_bundle,
                        &self.key_store.wai_ear_key,
                        &mut self.key_store.leaf_signers,
                        &self.key_store.as_intermediate_credentials,
                        &self.contacts,
                        &self.key_store.signing_key.credential(),
                    );
                    freshly_joined_groups.push(group_id.clone());

                    let attributes = ConversationAttributes {
                        title: "New conversation".to_string(),
                    };
                    let conversation_id = self
                        .conversation_store
                        .create_group_conversation(group_id, attributes);
                    self.notification_hub
                        .dispatch_conversation_notification(&conversation_id);
                }
                ExtractedQsQueueMessagePayload::MlsMessage(mls_message) => {
                    let protocol_message: ProtocolMessage = match mls_message.extract() {
                        MlsMessageInBody::PublicMessage(handshake_message) =>
                            handshake_message.into(),
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
                    let Some(group) = self.group_store.get_group_mut(group_id)
                        else {
                            return Err(CorelibError::GroupStore(GroupStoreError::UnknownGroup))
                        };
                    let (processed_message, we_were_removed, sender_credential) = group
                        .process_message(
                            &self.crypto_backend,
                            protocol_message,
                            &self.key_store.as_intermediate_credentials,
                        )
                        .unwrap();

                    let sender = processed_message.sender().clone();
                    let aad = processed_message.authenticated_data().to_vec();
                    let conversation_messages = match processed_message.into_content() {
                        ProcessedMessageContent::ApplicationMessage(application_message) => {
                            application_message_to_conversation_messages(
                                &sender_credential,
                                application_message,
                            )
                        }
                        ProcessedMessageContent::ProposalMessage(proposal) => {
                            // For now, we don't to anything here. The proposal
                            // was processed by the MLS group and will be
                            // committed with the next commit.
                            group.store_proposal(*proposal);
                            vec![]
                        }
                        ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                            // If a client joined externally, we
                            // check if the group belongs to an
                            // unconfirmed conversation.
                            if let ConversationType::UnconfirmedConnection(user_name) = &self
                                .conversation_store
                                .conversation(&conversation_id)
                                .unwrap()
                                .conversation_type
                            {
                                // Check if it was an external commit and if the user name matches
                                if matches!(sender, Sender::NewMemberCommit)
                                    && &sender_credential.identity().user_name().as_bytes()
                                        == user_name
                                {
                                    // Load up the partial contact and decrypt the friendship package
                                    let partial_contact = self
                                        .partial_contacts
                                        .remove(&(user_name.clone().into()))
                                        .unwrap();

                                    // This is a bit annoying,
                                    // since we already
                                    // de-serialized this in the
                                    // group processing
                                    // function, but we need the
                                    // encrypted friendship
                                    // package here.
                                    let encrypted_friendship_package =
                                        if let InfraAadPayload::JoinConnectionGroup(payload) =
                                            InfraAadMessage::tls_deserialize_exact(&aad)
                                                .unwrap()
                                                .into_payload()
                                        {
                                            payload.encrypted_friendship_package
                                        } else {
                                            panic!("Unexpected AAD payload")
                                        };

                                    let friendship_package = FriendshipPackage::decrypt(
                                        &partial_contact.friendship_package_ear_key,
                                        &encrypted_friendship_package,
                                    )
                                    .unwrap();
                                    // We also need to get the add infos
                                    let mut add_infos = vec![];
                                    for _ in 0..5 {
                                        let key_package_batch_response = self
                                            .api_client
                                            .qs_key_package_batch(
                                                friendship_package.friendship_token.clone(),
                                                friendship_package.add_package_ear_key.clone(),
                                            )
                                            .await
                                            .unwrap();
                                        let key_packages: Vec<(KeyPackage, SignatureEarKey)> =
                                            key_package_batch_response
                                                .add_packages
                                                .into_iter()
                                                .map(|add_package| {
                                                    let verified_add_package = add_package
                                                        .validate(
                                                            self.crypto_backend.crypto(),
                                                            ProtocolVersion::default(),
                                                        )
                                                        .unwrap();
                                                    let key_package =
                                                        verified_add_package.key_package().clone();
                                                    let sek = SignatureEarKey::decrypt(
                                                        &friendship_package
                                                            .signature_ear_key_wrapper_key,
                                                        verified_add_package
                                                            .encrypted_signature_ear_key(),
                                                    )
                                                    .unwrap();
                                                    (key_package, sek)
                                                })
                                                .collect();
                                        let key_package_batch = key_package_batch_response
                                            .key_package_batch
                                            .verify(&self.key_store.qs_verifying_key)
                                            .unwrap();
                                        let add_info = ContactAddInfos {
                                            key_package_batch,
                                            key_packages,
                                        };
                                        add_infos.push(add_info);
                                    }
                                    // Now we can turn the partial contact into a full one.
                                    let contact = partial_contact.into_contact(
                                        friendship_package,
                                        add_infos,
                                        sender_credential.clone(),
                                    );
                                    // And add it to our list of contacts
                                    self.contacts.insert(user_name.clone().into(), contact);
                                    // Finally, we can turn the conversation type to a full connection group
                                    self.conversation_store
                                        .confirm_connection_conversation(&conversation_id);
                                }
                            }
                            // If we were removed, we set the group to inactive.
                            if we_were_removed {
                                let past_members = group
                                    .members()
                                    .into_iter()
                                    .map(|user_name| user_name.to_string())
                                    .collect::<Vec<_>>();
                                self.conversation_store
                                    .set_inactive(&conversation_id, &past_members);
                            }
                            group
                                .merge_pending_commit(&self.crypto_backend, *staged_commit)
                                .unwrap()
                        }
                        ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                            unimplemented!()
                        }
                    };
                    // If we got until here, the message was deemed valid and we can apply the diff.
                    self.send_off_notifications(&conversation_id, conversation_messages)?;
                }
            }
        }

        // After joining, we need to set our user auth keys.
        for group_id in freshly_joined_groups {
            let group = self.group_store.get_group_mut(&group_id).unwrap();
            let params = group.update_user_key(&self.crypto_backend);
            self.api_client
                .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
                .await
                .unwrap();
            // Instead of using the conversation messages, we just
            // dispatch a conversation notification.
            let _conversation_messages = group
                .merge_pending_commit(&self.crypto_backend, None)
                .unwrap();
        }
        Ok(())
    }

    pub async fn process_as_messages(
        &mut self,
        message_ciphertexts: Vec<QueueMessage>,
    ) -> Result<(), CorelibError> {
        // Decrypt received message.
        let messages: Vec<ExtractedAsQueueMessagePayload> = message_ciphertexts
            .into_iter()
            .map(|message_ciphertext| {
                self.key_store
                    .as_queue_ratchet
                    .decrypt(message_ciphertext)
                    .unwrap()
                    .extract()
                    .unwrap()
            })
            .collect();

        let notification_messages = vec![];
        for message in messages {
            match message {
                ExtractedAsQueueMessagePayload::EncryptedConnectionEstablishmentPackage(ecep) => {
                    let cep_in = ConnectionEstablishmentPackageIn::decrypt(
                        ecep,
                        &self.key_store.connection_decryption_key,
                        &[],
                        &[],
                    )
                    .unwrap();
                    let cep_tbs = cep_in.verify_all(&self.key_store.as_intermediate_credentials);
                    // We create a new group and signal that fact to the user,
                    // so the user can decide if they want to accept the
                    // connection.

                    // Fetch external commit information.
                    let eci: ExternalCommitInfoIn = self
                        .api_client
                        .ds_connection_group_info(
                            cep_tbs.connection_group_id.clone(),
                            &cep_tbs.connection_group_ear_key,
                        )
                        .await
                        .unwrap();

                    let signature_ear_key = SignatureEarKey::random().unwrap();
                    let leaf_signer = InfraCredentialSigningKey::generate(
                        &self.key_store.signing_key,
                        &signature_ear_key,
                    );
                    let esek = signature_ear_key
                        .encrypt(&cep_tbs.connection_group_signature_ear_key_wrapper_key)
                        .unwrap();

                    let encrypted_friendship_package = FriendshipPackage {
                        friendship_token: self.key_store.friendship_token.clone(),
                        add_package_ear_key: self.key_store.add_package_ear_key.clone(),
                        client_credential_ear_key: self.key_store.client_credential_ear_key.clone(),
                        signature_ear_key_wrapper_key: self
                            .key_store
                            .signature_ear_key_wrapper_key
                            .clone(),
                        wai_ear_key: self.key_store.wai_ear_key.clone(),
                    }
                    .encrypt(&cep_tbs.friendship_package_ear_key)
                    .unwrap();
                    let ecc = self
                        .key_store
                        .signing_key
                        .credential()
                        .encrypt(&cep_tbs.connection_group_credential_key)
                        .unwrap();

                    let aad = InfraAadPayload::JoinConnectionGroup(JoinConnectionGroupParamsAad {
                        encrypted_client_information: (ecc, esek),
                        encrypted_friendship_package,
                    })
                    .into();

                    let (group, commit, group_info) = Group::join_group_externally(
                        &self.crypto_backend,
                        eci,
                        leaf_signer,
                        signature_ear_key,
                        cep_tbs.connection_group_ear_key.clone(),
                        cep_tbs.connection_group_signature_ear_key_wrapper_key,
                        cep_tbs.connection_group_credential_key,
                        &self.key_store.as_intermediate_credentials,
                        aad,
                        self.key_store.signing_key.credential(),
                    )
                    .unwrap();
                    let user_name = cep_tbs.sender_client_credential.identity().user_name();
                    let conversation_id = self.conversation_store.create_connection_conversation(
                        group.group_id(),
                        user_name.clone(),
                        ConversationAttributes {
                            title: user_name.to_string(),
                        },
                    );
                    // TODO: For now, we automatically confirm conversations.
                    self.conversation_store
                        .confirm_connection_conversation(&conversation_id);
                    // Fetch a keypackage for our new contact.
                    // TODO: For now, one is enough.
                    let response = self
                        .api_client
                        .qs_key_package_batch(
                            cep_tbs.friendship_package.friendship_token.clone(),
                            cep_tbs.friendship_package.add_package_ear_key.clone(),
                        )
                        .await
                        .unwrap();
                    let key_packages: Vec<(KeyPackage, SignatureEarKey)> = response
                        .add_packages
                        .into_iter()
                        .map(|add_package| {
                            let validated_add_package = add_package
                                .validate(self.crypto_backend.crypto(), ProtocolVersion::default())
                                .unwrap();
                            let key_package = validated_add_package.key_package().clone();
                            let sek = SignatureEarKey::decrypt(
                                &cep_tbs.friendship_package.signature_ear_key_wrapper_key,
                                validated_add_package.encrypted_signature_ear_key(),
                            )
                            .unwrap();
                            (key_package, sek)
                        })
                        .collect();
                    let add_info = ContactAddInfos {
                        key_packages,
                        key_package_batch: response
                            .key_package_batch
                            .verify(&self.key_store.qs_verifying_key)
                            .unwrap(),
                    };

                    let contact = PartialContact {
                        user_name: user_name.clone(),
                        conversation_id,
                        friendship_package_ear_key: cep_tbs.friendship_package_ear_key,
                    }
                    .into_contact(
                        cep_tbs.friendship_package,
                        vec![add_info],
                        cep_tbs.sender_client_credential,
                    );
                    self.contacts.insert(user_name, contact);
                    // TODO: Send conversation message to UI.

                    let sealed_reference = ClientConfig {
                        client_id: self.qs_client_id.clone(),
                        push_token_ear_key: Some(self.key_store.push_token_ear_key.clone()),
                    }
                    .encrypt(
                        &self.key_store.qs_client_id_encryption_key,
                        &[],
                        &[],
                    );
                    let qs_client_reference = QsClientReference {
                        client_homeserver_domain: Fqdn {},
                        sealed_reference,
                    };
                    // Send the confirmation by way of commit and group info to the DS.
                    self.api_client
                        .ds_join_connection_group(
                            commit,
                            group_info,
                            qs_client_reference,
                            // We unwrap here, because this is an external commit.
                            group.user_auth_key().unwrap(),
                            &cep_tbs.connection_group_ear_key,
                        )
                        .await
                        .unwrap();
                    self.group_store.store_group(group).unwrap();
                }
            }
        }
        // TODO: We notify in bulk here. We might want to change this in the future.
        for notification_message in notification_messages.clone() {
            self.notification_hub
                .dispatch_message_notification(notification_message);
        }
        Ok(())
    }

    /// Get existing conversations
    pub fn get_conversations(&self) -> Vec<Conversation> {
        self.conversation_store.conversations()
    }

    pub fn conversation(&self, conversation_id: &Uuid) -> Option<&Conversation> {
        self.conversation_store.conversation(conversation_id)
    }

    pub fn get_messages(&self, conversation_id: &Uuid, last_n: usize) -> Vec<ConversationMessage> {
        self.conversation_store.messages(conversation_id, last_n)
    }
}
