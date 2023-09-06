// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{bail, Result};
use phnxbackend::{
    crypto::{ear::EarDecryptable, hpke::HpkeDecryptable},
    ds::api::QualifiedGroupId,
    messages::{
        client_as::ExtractedAsQueueMessagePayload,
        client_as_out::ConnectionEstablishmentPackageIn,
        client_ds::{
            ExtractedQsQueueMessagePayload, InfraAadMessage, InfraAadPayload,
            JoinConnectionGroupParamsAad,
        },
        client_ds_out::ExternalCommitInfoIn,
        QueueMessage,
    },
};
use tls_codec::DeserializeBytes;

use super::key_store::{PersistableAsQueueRatchet, QueueRatchetType};
use super::*;

impl<T: Notifiable> SelfUser<T> {
    /// Process received messages by group. This function is meant to be called
    /// with messages received from the QS queue.
    pub async fn process_qs_messages(
        &mut self,
        message_ciphertexts: Vec<QueueMessage>,
    ) -> Result<()> {
        // Decrypt received message.
        let messages: Vec<ExtractedQsQueueMessagePayload> = message_ciphertexts
            .into_iter()
            .map(|message_ciphertext| {
                let mut qs_queue_ratchet =
                    PersistableQsQueueRatchet::load(&self.as_client_id(), &QueueRatchetType::Qs)?;
                let payload = qs_queue_ratchet.decrypt(message_ciphertext)?;
                Ok(payload.extract()?)
            })
            .collect::<Result<Vec<_>>>()?;

        // TODO: We should verify whether the messages are valid infra messages, i.e.
        // if it doesn't mix requests, etc. I think the DS already does some of this
        // and we might be able to re-use code.

        // Keep track of freshly joined groups s.t. we can later update our user auth keys.
        let mut freshly_joined_groups = vec![];
        for message in messages {
            match message {
                ExtractedQsQueueMessagePayload::WelcomeBundle(welcome_bundle) => {
                    let group = Group::join_group(
                        &self.crypto_backend,
                        welcome_bundle,
                        &self.key_store.wai_ear_key,
                        // TODO: For now, I'm passing the ApiClients in here
                        // s.t. the group can fetch AS Credentials if it needs
                        // to. In the future, it would be great if we could have
                        // multiple references to the API clients flying around,
                        // for example one in the ASCredentials store itself.
                        &mut self.api_clients,
                        &self.key_store.signing_key.credential().identity(),
                    )
                    .await?;
                    let group_id = group.group_id();
                    freshly_joined_groups.push(group_id.clone());

                    let attributes = ConversationAttributes {
                        title: "New conversation".to_string(),
                    };
                    let conversation = Conversation::create_group_conversation(
                        &self.as_client_id(),
                        group_id.clone(),
                        attributes,
                    )?;
                    self.dispatch_conversation_notification(conversation.id());
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
                        MlsMessageInBody::GroupInfo(_) | MlsMessageInBody::KeyPackage(_) => bail!("Unexpected message type"),
                    };
                    let group_id = protocol_message.group_id();
                    let group_id_bytes: GroupIdBytes = group_id.clone().into();
                    let conversation_id =
                        Conversation::load_secondary(&self.as_client_id(), &group_id_bytes)?
                            .id
                            .as_uuid();

                    let mut group = Group::load(&self.as_client_id(), protocol_message.group_id())?;
                    let (processed_message, we_were_removed, sender_credential) = group
                        .process_message(
                            &self.crypto_backend,
                            protocol_message,
                            &mut self.api_clients,
                        )
                        .await?;

                    let sender = processed_message.sender().clone();
                    let aad = processed_message.authenticated_data().to_vec();
                    let conversation_messages = match processed_message.into_content() {
                        ProcessedMessageContent::ApplicationMessage(application_message) => {
                            vec![GroupMessage::from_application_message(
                                &sender_credential,
                                application_message,
                            )?]
                        }
                        ProcessedMessageContent::ProposalMessage(proposal) => {
                            // For now, we don't to anything here. The proposal
                            // was processed by the MLS group and will be
                            // committed with the next commit.
                            group.store_proposal(*proposal)?;
                            vec![]
                        }
                        ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                            // If a client joined externally, we check if the
                            // group belongs to an unconfirmed conversation.
                            let mut conversation =
                                Conversation::load(&self.as_client_id(), &conversation_id.into())?;
                            if let ConversationType::UnconfirmedConnection(ref user_name) =
                                conversation.conversation_type
                            {
                                let user_name = user_name.clone().into();
                                // Check if it was an external commit and if the user name matches
                                if !matches!(sender, Sender::NewMemberCommit)
                                    && sender_credential.identity().user_name() == user_name
                                {
                                    // TODO: Handle the fact that an unexpected user joined the connection group.
                                }
                                // Load up the partial contact and decrypt the
                                // friendship package
                                let partial_contact =
                                    PartialContact::load(&self.as_client_id(), &user_name)?;

                                // This is a bit annoying, since we already
                                // de-serialized this in the group processing
                                // function, but we need the encrypted
                                // friendship package here.
                                let encrypted_friendship_package =
                                    if let InfraAadPayload::JoinConnectionGroup(payload) =
                                        InfraAadMessage::tls_deserialize_exact(&aad)?.into_payload()
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
                                                    self.crypto_backend.crypto(),
                                                    ProtocolVersion::default(),
                                                )?;
                                                let key_package =
                                                    verified_add_package.key_package().clone();
                                                let sek = SignatureEarKey::decrypt(
                                                    &friendship_package
                                                        .signature_ear_key_wrapper_key,
                                                    verified_add_package
                                                        .encrypted_signature_ear_key(),
                                                )?;
                                                Ok((key_package, sek))
                                            })
                                            .collect::<Result<Vec<_>>>()?;
                                    let qs_verifying_key = PersistableQsVerifyingKey::get(
                                        &self.as_client_id(),
                                        &mut self.api_clients,
                                        &user_name.domain(),
                                    )
                                    .await?;
                                    let key_package_batch = key_package_batch_response
                                        .key_package_batch
                                        .verify(&qs_verifying_key)?;
                                    let add_info = ContactAddInfos {
                                        key_package_batch,
                                        key_packages,
                                    };
                                    add_infos.push(add_info);
                                }
                                // Now we can turn the partial contact into a full one.
                                partial_contact.into_contact_and_persist(
                                    &self.as_client_id(),
                                    friendship_package,
                                    add_infos,
                                    sender_credential.clone(),
                                )?;
                                // Finally, we can turn the conversation type to a full connection group
                                conversation.confirm()?;
                                // And notify the application
                                self.dispatch_conversation_notification(conversation_id);
                            }
                            // If we were removed, we set the group to inactive.
                            if we_were_removed {
                                let past_members = group
                                    .members()
                                    .into_iter()
                                    .map(|user_name| user_name.to_string())
                                    .collect::<Vec<_>>();
                                conversation.set_inactive(&past_members)?;
                            }
                            group.merge_pending_commit(&self.crypto_backend, *staged_commit)?
                        }
                        ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                            unimplemented!()
                        }
                    };
                    // If we got until here, the message was deemed valid and we can apply the diff.
                    self.dispatch_message_notifications(conversation_id, conversation_messages)?;
                }
            }
        }

        // After joining, we need to set our user auth keys.
        for group_id in freshly_joined_groups {
            let mut group = Group::load(&self.as_client_id(), &group_id)?;
            let params = group.update_user_key(&self.crypto_backend)?;
            let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice()).unwrap();
            self.api_clients
                .get(&qgid.owning_domain)?
                .ds_update_client(params, group.group_state_ear_key(), group.leaf_signer())
                .await?;
            // Instead of using the conversation messages, we just
            // dispatch a conversation notification.
            let _conversation_messages = group.merge_pending_commit(&self.crypto_backend, None)?;
        }
        Ok(())
    }

    pub async fn process_as_messages(
        &mut self,
        message_ciphertexts: Vec<QueueMessage>,
    ) -> Result<()> {
        // Decrypt received message.
        let messages: Vec<ExtractedAsQueueMessagePayload> = message_ciphertexts
            .into_iter()
            .map(|message_ciphertext| {
                let mut as_queue_ratchet =
                    PersistableAsQueueRatchet::load(&self.as_client_id(), &QueueRatchetType::As)?;
                let payload = as_queue_ratchet.decrypt(message_ciphertext)?;
                Ok(payload.extract()?)
            })
            .collect::<Result<Vec<_>>>()?;

        let mut conversations_with_messages = vec![];
        for message in messages {
            match message {
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

                    let as_intermediate_credential = PersistableAsIntermediateCredential::get(
                        &self.as_client_id(),
                        &mut self.api_clients,
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
                    let qgid = QualifiedGroupId::tls_deserialize_exact(
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

                    let (group, commit, group_info) = Group::join_group_externally(
                        &self.crypto_backend,
                        eci,
                        leaf_signer,
                        signature_ear_key,
                        cep_tbs.connection_group_ear_key.clone(),
                        cep_tbs.connection_group_signature_ear_key_wrapper_key,
                        cep_tbs.connection_group_credential_key,
                        &mut self.api_clients,
                        aad,
                        self.key_store.signing_key.credential(),
                    )
                    .await?;
                    let user_name = cep_tbs.sender_client_credential.identity().user_name();
                    let mut conversation = Conversation::create_connection_conversation(
                        &self.as_client_id(),
                        group.group_id().clone(),
                        user_name.clone(),
                        ConversationAttributes {
                            title: user_name.to_string(),
                        },
                    )?;
                    // TODO: For now, we automatically confirm conversations.
                    conversation.confirm()?;
                    conversations_with_messages.push(conversation.id());

                    PartialContact::new(
                        &self.as_client_id(),
                        user_name.clone(),
                        conversation.id(),
                        cep_tbs.friendship_package_ear_key,
                    )?
                    .into_contact_and_persist(
                        &self.as_client_id(),
                        cep_tbs.friendship_package,
                        vec![],
                        cep_tbs.sender_client_credential,
                    )?;

                    // Fetch a few KeyPackages for our new contact.
                    self.get_key_packages(&user_name).await?;
                    // TODO: Send conversation message to UI.

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
                }
            }
        }
        // TODO: We notify in bulk here. We might want to change this in the future.
        for conversation_id in conversations_with_messages {
            self.dispatch_conversation_notification(conversation_id)
        }
        Ok(())
    }

    pub fn conversation(&self, conversation_id: Uuid) -> Option<Conversation> {
        Conversation::load(&self.as_client_id(), &conversation_id.into()).ok()
    }

    pub fn get_messages(
        &self,
        conversation_id: Uuid,
        last_n: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let messages = ConversationMessage::load_multiple_secondary(
            &self.as_client_id(),
            &conversation_id.into(),
        )?;

        if last_n >= messages.len() {
            Ok(messages)
        } else {
            let (_left, right) = messages.split_at(messages.len() - last_n);
            Ok(right.to_vec())
        }
    }
}
