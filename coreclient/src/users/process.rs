// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{bail, Result};
use phnxbackend::crypto::ear::EarDecryptable;
use phnxbackend::crypto::hpke::HpkeDecryptable;
use phnxbackend::ds::api::QualifiedGroupId;
use phnxbackend::messages::client_as::ExtractedAsQueueMessagePayload;
use phnxbackend::messages::client_as_out::ConnectionEstablishmentPackageIn;
use phnxbackend::messages::client_ds::{
    ExtractedQsQueueMessagePayload, InfraAadMessage, InfraAadPayload, JoinConnectionGroupParamsAad,
};
use phnxbackend::messages::client_ds_out::ExternalCommitInfoIn;
use phnxbackend::messages::QueueMessage;
use tls_codec::DeserializeBytes;

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
                let message = self
                    .key_store
                    .qs_queue_ratchet
                    .decrypt(message_ciphertext)?
                    .extract()?;
                Ok(message)
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
                    let group_id = self
                        .group_store
                        .join_group(
                            &self.crypto_backend,
                            welcome_bundle,
                            &self.key_store.wai_ear_key,
                            &mut self.key_store.leaf_signers,
                            // TODO: For now, I'm passing the ApiClients in here
                            // s.t. the group can fetch AS Credentials if it needs
                            // to. In the future, it would be great if we could have
                            // multiple references to the API clients flying around,
                            // for example one in the ASCredentials store itself.
                            &mut self.api_clients,
                            &mut self.key_store.as_credentials,
                            &self.contacts,
                        )
                        .await?;
                    freshly_joined_groups.push(group_id.clone());

                    let attributes = ConversationAttributes {
                        title: "New conversation".to_string(),
                    };
                    let conversation_id = self
                        .conversation_store
                        .create_group_conversation(group_id, attributes);
                    self.notification_hub
                        .dispatch_conversation_notification(conversation_id);
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
                    let conversation_id = self
                        .conversation_store
                        .conversation_by_group_id(group_id)
                        .ok_or(anyhow!("Can't find conversation for the given group id"))?
                        .id
                        .as_uuid();
                    let Some(group) = self.group_store.get_group_mut(group_id)
                        else {
                            bail!("Unknown group")
                        };
                    let (processed_message, we_were_removed, sender_credential) = group
                        .process_message(
                            &self.crypto_backend,
                            protocol_message,
                            &mut self.api_clients,
                            &mut self.key_store.as_credentials,
                        )
                        .await?;

                    let sender = processed_message.sender().clone();
                    let aad = processed_message.authenticated_data().to_vec();
                    let conversation_messages = match processed_message.into_content() {
                        ProcessedMessageContent::ApplicationMessage(application_message) => {
                            application_message_to_conversation_messages(
                                &sender_credential,
                                application_message,
                            )?
                        }
                        ProcessedMessageContent::ProposalMessage(proposal) => {
                            // For now, we don't to anything here. The proposal
                            // was processed by the MLS group and will be
                            // committed with the next commit.
                            group.store_proposal(*proposal);
                            vec![]
                        }
                        ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                            // If a client joined externally, we check if the
                            // group belongs to an unconfirmed conversation.
                            if let ConversationType::UnconfirmedConnection(user_name) = &self
                                .conversation_store
                                .conversation(conversation_id)
                                .ok_or(anyhow!("Can't find conversation for the given id"))?
                                .conversation_type
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
                                let partial_contact = self.partial_contacts.remove(&user_name).ok_or(anyhow!("Unknown sender: Can't find partial contact while processing connection request"))?;

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
                                    let qs_verifying_key = if let Some(qs_verifying_key) =
                                        self.key_store.qs_verifying_keys.get(&user_name.domain())
                                    {
                                        qs_verifying_key
                                    } else {
                                        let qs_verifying_key = self
                                            .api_clients
                                            .get(&user_name.domain())?
                                            .qs_verifying_key()
                                            .await?;
                                        self.key_store.qs_verifying_keys.insert(
                                            user_name.domain().clone(),
                                            qs_verifying_key.verifying_key,
                                        );
                                        self.key_store
                                            .qs_verifying_keys
                                            .get(&user_name.domain())
                                            .ok_or(anyhow!("Error fetching QS veryfing key"))?
                                    };
                                    let key_package_batch = key_package_batch_response
                                        .key_package_batch
                                        .verify(qs_verifying_key)?;
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
                                // And notify the application
                                self.notification_hub
                                    .dispatch_conversation_notification(conversation_id);
                            }
                            // If we were removed, we set the group to inactive.
                            if we_were_removed {
                                let past_members = group
                                    .members()
                                    .into_iter()
                                    .map(|user_name| user_name.to_string())
                                    .collect::<Vec<_>>();
                                self.conversation_store
                                    .set_inactive(conversation_id, &past_members);
                            }
                            group.merge_pending_commit(&self.crypto_backend, *staged_commit)?
                        }
                        ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                            unimplemented!()
                        }
                    };
                    // If we got until here, the message was deemed valid and we can apply the diff.
                    self.send_off_notifications(conversation_id, conversation_messages)?;
                }
            }
        }

        // After joining, we need to set our user auth keys.
        for group_id in freshly_joined_groups {
            let group = self
                .group_store
                .get_group_mut(&group_id)
                .ok_or(anyhow!("Error finding freshly created group"))?;
            let params = group.update_user_key(&self.crypto_backend)?;
            let qgid = QualifiedGroupId::tls_deserialize_exact(group_id.as_slice())?;
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
                let message = self
                    .key_store
                    .as_queue_ratchet
                    .decrypt(message_ciphertext)?
                    .extract()?;
                Ok(message)
            })
            .collect::<Result<Vec<_>>>()?;

        let mut notification_messages = vec![];
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

                    let as_intermediate_credential = self
                        .key_store
                        .as_credentials
                        .get(
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
                        &mut self.key_store.as_credentials,
                        &mut self.api_clients,
                        aad,
                        self.key_store.signing_key.credential(),
                    )
                    .await?;
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

                    notification_messages.push(conversation_id);

                    let contact = PartialContact {
                        user_name: user_name.clone(),
                        conversation_id,
                        friendship_package_ear_key: cep_tbs.friendship_package_ear_key,
                    }
                    .into_contact(
                        cep_tbs.friendship_package,
                        vec![],
                        cep_tbs.sender_client_credential,
                    );

                    self.contacts.insert(user_name.clone(), contact);
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
                    self.group_store.store_group(group)?;
                }
            }
        }
        // TODO: We notify in bulk here. We might want to change this in the future.
        for notification_message in notification_messages {
            self.notification_hub
                .dispatch_conversation_notification(notification_message);
        }
        Ok(())
    }

    /// Get existing conversations
    pub fn get_conversations(&self) -> Vec<Conversation> {
        self.conversation_store.conversations()
    }

    pub fn conversation(&self, conversation_id: Uuid) -> Option<&Conversation> {
        self.conversation_store.conversation(conversation_id)
    }

    pub fn get_messages(&self, conversation_id: Uuid, last_n: usize) -> Vec<ConversationMessage> {
        self.conversation_store.messages(conversation_id, last_n)
    }

    pub(super) async fn qs_verifying_key(&mut self, domain: &Fqdn) -> Result<&QsVerifyingKey> {
        if !self.key_store.qs_verifying_keys.contains_key(domain) {
            let qs_verifying_key = self.api_clients.get(domain)?.qs_verifying_key().await?;
            self.key_store
                .qs_verifying_keys
                .insert(domain.clone(), qs_verifying_key.verifying_key);
        }
        self.key_store
            .qs_verifying_keys
            .get(domain)
            .ok_or(anyhow!("Can't find QS verifying key for the given domain"))
    }
}
