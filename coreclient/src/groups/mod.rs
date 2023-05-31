// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod error;
pub(crate) mod store;

use ds_lib::GroupMessage;
pub(crate) use error::*;
use openmls_memory_keystore::MemoryKeyStore;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::signatures::Signer;
use phnxbackend::{
    auth_service::credentials::keys::{generate_signature_keypair, ClientSigningKey},
    crypto::signatures::traits::SigningKey,
};
pub(crate) use store::*;

use crate::{conversations::*, types::MessageContentType, types::*};
use std::collections::HashMap;

use openmls::prelude::{group_info::GroupInfo, *};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct InfraCredentialSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: InfraCredential,
}

// 30 days lifetime in seconds
pub(crate) const DEFAULT_INFRA_CREDENTIAL_LIFETIME: u64 = 30 * 24 * 60 * 60;

impl InfraCredentialSigningKey {
    pub fn generate(client_signer: &ClientSigningKey) -> Self {
        let keypair = generate_signature_keypair().unwrap();
        let identity = OpenMlsRustCrypto::default().rand().random_vec(32).unwrap();
        let credential = InfraCredential::new(
            identity,
            // 30 days lifetime
            Lifetime::new(DEFAULT_INFRA_CREDENTIAL_LIFETIME),
            SignatureScheme::ED25519,
            keypair.1.clone().into(),
        );
        Self {
            signing_key_bytes: keypair.0,
            credential,
        }
    }

    pub(crate) fn credential(&self) -> &InfraCredential {
        &self.credential
    }
}

impl SigningKey for InfraCredentialSigningKey {}

impl AsRef<[u8]> for InfraCredentialSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl Signer for InfraCredentialSigningKey {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, Error> {
        <Self as SigningKey>::sign(self, payload)
            .map_err(|_| Error::SigningError)
            .map(|s| s.into_bytes())
    }

    fn signature_scheme(&self) -> SignatureScheme {
        self.credential.credential_ciphersuite()
    }
}

#[derive(Debug)]
pub(crate) struct Group {
    group_id: Uuid,
    leaf_signer: InfraCredentialSigningKey,
    mls_group: MlsGroup,
}

impl Group {
    /// Create a group.
    pub fn create_group(backend: &impl OpenMlsCryptoProvider, signer: &ClientSigningKey) -> Self {
        let group_id = Uuid::new_v4();

        let leaf_signer = InfraCredentialSigningKey::generate(signer);

        let mls_group_config = MlsGroupConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();

        let credential_with_key = CredentialWithKey {
            credential: Credential::from(leaf_signer.credential.clone()),
            signature_key: leaf_signer.credential.verifying_key().clone(),
        };

        let mls_group = MlsGroup::new_with_group_id(
            backend,
            &leaf_signer,
            &mls_group_config,
            GroupId::from_slice(group_id.as_bytes()),
            credential_with_key,
        )
        .unwrap();

        Group {
            group_id,
            leaf_signer,
            mls_group,
        }
    }

    /// Join a group with the provided welcome message. Returns the group name.
    pub(crate) fn join_group(
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        welcome: Welcome,
        leaf_signers: &mut HashMap<SignaturePublicKey, InfraCredentialSigningKey>,
    ) -> Result<Self, GroupOperationError> {
        //log::debug!("{} joining group ...", self_user.username);

        let mls_group_config = MlsGroupConfig::default();
        let mls_group = match MlsGroup::new_from_welcome(
            backend,
            &mls_group_config,
            welcome,
            None, /* no public tree here, has to be in the extension */
        ) {
            Ok(g) => g,
            Err(e) => {
                let s = format!("Error creating group from Welcome: {:?}", e);
                log::info!("{}", s);
                return Err(GroupOperationError::WelcomeError(e));
            }
        };

        let verifying_key = mls_group.own_leaf_node().unwrap().signature_key();
        let leaf_signer = leaf_signers.remove(verifying_key).unwrap();

        let mut members = Vec::new();
        for member in mls_group.members() {
            let identity = member.credential.identity().to_vec();
            members.push(identity);
        }

        let group = Group {
            group_id: UuidBytes::from_bytes(mls_group.group_id().as_slice()).as_uuid(),
            mls_group,
            leaf_signer,
        };

        Ok(group)
    }

    /// Process inbound message
    pub(crate) fn process_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider,
        message: MlsMessageIn,
    ) -> Result<ProcessedMessage, GroupOperationError> {
        Ok(self.mls_group.process_message(backend, message)?)
    }

    /// Invite user with the given name to the group.
    ///
    /// Returns the Commit, as well as the Welcome message as a tuple in that
    /// order.
    pub(crate) fn invite<'a>(
        &'a mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        signer: &impl Signer,
        credential_with_key: &CredentialWithKey,
        key_package: KeyPackage,
    ) -> Result<(MlsMessageOut, MlsMessageOut, Option<GroupInfo>), GroupOperationError> {
        Ok(self
            .mls_group
            .add_members(backend, signer, &[key_package])?)
    }

    /// Merge the pending commit
    pub(crate) fn merge_pending_commit(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
    ) -> Result<(), GroupOperationError> {
        Ok(self.mls_group.merge_pending_commit(backend)?)
    }

    /// Get a list of clients in the group to send messages to.
    fn recipients(&self, credential_with_key: &CredentialWithKey) -> Vec<Vec<u8>> {
        let recipients: Vec<Vec<u8>> = self
            .mls_group
            .members()
            .filter_map(|kp| {
                if credential_with_key.credential.identity() != kp.credential.identity() {
                    Some(kp.credential.identity().to_vec())
                } else {
                    None
                }
            })
            .collect();
        recipients
    }

    /// Send an application message to the group.
    pub fn create_message(
        &mut self,
        backend: &impl OpenMlsCryptoProvider<KeyStoreProvider = MemoryKeyStore>,
        signer: &impl Signer,
        credential_with_key: &CredentialWithKey,
        msg: &str,
    ) -> Result<GroupMessage, GroupOperationError> {
        let mls_message_out = self
            .mls_group
            .create_message(backend, signer, msg.as_bytes())?;

        Ok(GroupMessage::new(
            mls_message_out.into(),
            &self.recipients(credential_with_key),
        ))
    }

    /// Get a reference to the group's group id.
    pub(crate) fn group_id(&self) -> Uuid {
        self.group_id
    }
}

pub(crate) fn application_message_to_conversation_messages(
    sender: &Credential,
    application_message: ApplicationMessage,
) -> Vec<ConversationMessage> {
    vec![new_conversation_message(Message::Content(ContentMessage {
        sender: String::from_utf8_lossy(sender.identity()).into(),
        content: MessageContentType::Text(TextMessage {
            message: String::from_utf8_lossy(&application_message.into_bytes()).into(),
        }),
    }))]
}

pub(crate) fn staged_commit_to_conversation_messages(
    sender: &Credential,
    staged_commit: &StagedCommit,
) -> Vec<ConversationMessage> {
    let adds: Vec<ConversationMessage> = staged_commit
        .add_proposals()
        .map(|staged_add_proposal| {
            let added_member = staged_add_proposal
                .add_proposal()
                .key_package()
                .leaf_node()
                .credential()
                .identity();
            let event_message = format!(
                "{} added {} to the conversation",
                String::from_utf8_lossy(sender.identity()),
                String::from_utf8_lossy(added_member)
            );
            event_message_from_string(event_message)
        })
        .collect();
    let mut removes: Vec<ConversationMessage> = staged_commit
        .remove_proposals()
        .map(|staged_remove_proposal| {
            let removed_member = staged_remove_proposal.remove_proposal().removed();
            let event_message = format!(
                "{} removed {:?} from the conversation",
                String::from_utf8_lossy(sender.identity()),
                removed_member
            );
            event_message_from_string(event_message)
        })
        .collect();
    let mut updates: Vec<ConversationMessage> = staged_commit
        .update_proposals()
        .map(|staged_update_proposal| {
            let updated_member = staged_update_proposal
                .update_proposal()
                .leaf_node()
                .credential()
                .identity();
            let event_message = format!("{} updated", String::from_utf8_lossy(updated_member),);
            event_message_from_string(event_message)
        })
        .collect();
    let mut events = adds;
    events.append(&mut removes);
    events.append(&mut updates);

    events
}

fn event_message_from_string(event_message: String) -> ConversationMessage {
    new_conversation_message(Message::Display(DisplayMessage {
        message: DisplayMessageType::System(SystemMessage {
            message: event_message,
        }),
    }))
}

/*
impl From<GroupEvent> for ConversationMessage {
    fn from(event: GroupEvent) -> ConversationMessage {
        let event_message = match &event {
            GroupEvent::MemberAdded(e) => Some(format!(
                "{} added {} to the conversation",
                String::from_utf8_lossy(e.sender().identity()),
                String::from_utf8_lossy(e.added_member().identity())
            )),
            GroupEvent::MemberRemoved(e) => match e.removal() {
                Removal::WeLeft => Some("We left the conversation".to_string()),
                Removal::TheyLeft(leaver) => Some(format!(
                    "{} left the conversation",
                    String::from_utf8_lossy(leaver.identity()),
                )),
                Removal::WeWereRemovedBy(remover) => Some(format!(
                    "{} removed us from the conversation",
                    String::from_utf8_lossy(remover.identity()),
                )),
                Removal::TheyWereRemovedBy(leaver, remover) => Some(format!(
                    "{} removed {} from the conversation",
                    String::from_utf8_lossy(remover.identity()),
                    String::from_utf8_lossy(leaver.identity())
                )),
            },
            GroupEvent::MemberUpdated(e) => Some(format!(
                "{} updated",
                String::from_utf8_lossy(e.updated_member().identity()),
            )),
            GroupEvent::PskReceived(_) => Some("PSK received".to_string()),
            GroupEvent::ReInit(_) => Some("ReInit received".to_string()),
            GroupEvent::ApplicationMessage(_) => None,
            openmls::group::GroupEvent::Error(e) => {
                Some(format!("Error occured in group: {:?}", e))
            }
        };

        let app_message = match &event {
            GroupEvent::ApplicationMessage(message) => {
                let content_message = ContentMessage {
                    sender: String::from_utf8_lossy(message.sender().identity()).into(),
                    content_type: Some(ContentType::TextMessage(TextMessage {
                        message: String::from_utf8_lossy(message.message()).into(),
                    })),
                };
                Some(content_message)
            }
            _ => None,
        };

        if let Some(event_message) = event_message {
            new_conversation_message(conversation_message::Message::DisplayMessage(
                DisplayMessage {
                    message: Some(display_message::Message::SystemMessage(SystemMessage {
                        content: event_message,
                    })),
                },
            ))
        } else {
            new_conversation_message(conversation_message::Message::ContentMessage(
                app_message.unwrap(),
            ))
        }
    }
}
*/
