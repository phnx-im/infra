// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use openmls_basic_credential::SignatureKeyPair;

use super::*;

pub(crate) const CIPHERSUITE: Ciphersuite =
    Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

pub struct SelfUser {
    pub(crate) crypto_backend: OpenMlsRustCrypto,
    pub(crate) username: String,
    pub(crate) credential_with_key: CredentialWithKey,
    pub(crate) signer: SignatureKeyPair,
    pub(crate) conversation_store: ConversationStore,
    pub(crate) group_store: GroupStore,
}

impl SelfUser {
    /// Create a new user with the given name and a fresh set of credentials.
    pub fn new(username: String) -> Self {
        let crypto_backend = OpenMlsRustCrypto::default();
        let credential =
            Credential::new(username.as_bytes().to_vec(), CredentialType::Basic).unwrap();
        let signer = SignatureKeyPair::new(SignatureScheme::from(CIPHERSUITE)).unwrap();
        signer.store(crypto_backend.key_store()).unwrap();

        Self {
            crypto_backend,
            username,
            credential_with_key: CredentialWithKey {
                credential,
                signature_key: signer.public().to_vec().into(),
            },
            signer,
            conversation_store: ConversationStore::default(),
            group_store: GroupStore::default(),
        }
    }

    pub(crate) fn generate_keypackage(&self) -> KeyPackage {
        KeyPackage::builder()
            .build(
                CryptoConfig {
                    ciphersuite: CIPHERSUITE,
                    version: ProtocolVersion::Mls10,
                },
                &self.crypto_backend,
                &self.signer,
                self.credential_with_key.clone(),
            )
            .unwrap()
    }

    /// Process received messages by group
    pub fn process_messages(
        &mut self,
        group_id: Uuid,
        messages: Vec<MlsMessageIn>,
    ) -> Result<Vec<DispatchedConversationMessage>, CorelibError> {
        let mut notification_messages = vec![];
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
