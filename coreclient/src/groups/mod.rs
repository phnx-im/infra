// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod error;
pub(crate) mod store;

use ds_lib::GroupMessage;
pub(crate) use error::*;
pub(crate) use store::*;

use crate::{backend::Backend, conversations::*, types::MessageContentType, types::*, users::*};
use std::collections::HashMap;

use openmls::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct Group {
    group_id: Uuid,
    mls_group: MlsGroup,
}

impl Group {
    /// Create a group.
    pub fn create_group(user: &mut SelfUser) -> Self {
        log::debug!("{} creates a group", user.username);
        let group_id = Uuid::new_v4();

        let mls_group_config = MlsGroupConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();

        let mls_group = MlsGroup::new_with_group_id(
            &user.crypto_backend,
            &user.signer,
            &mls_group_config,
            GroupId::from_slice(group_id.as_bytes()),
            user.credential_with_key.clone(),
        )
        .unwrap();

        Group {
            group_id,
            mls_group,
        }
    }

    /// Join a group with the provided welcome message. Returns the group name.
    pub(crate) fn join_group(
        self_user: &SelfUser,
        welcome: Welcome,
    ) -> Result<Self, GroupOperationError> {
        log::debug!("{} joining group ...", self_user.username);

        let mls_group_config = MlsGroupConfig::default();
        let mls_group = match MlsGroup::new_from_welcome(
            &self_user.crypto_backend,
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

        let mut members = Vec::new();
        for member in mls_group.members() {
            let identity = member.credential.identity().to_vec();
            members.push(identity);
        }

        let group = Group {
            group_id: UuidBytes::from_bytes(mls_group.group_id().as_slice()).as_uuid(),
            mls_group,
        };

        Ok(group)
    }

    /// Process inbound message
    pub(crate) fn process_message(
        &mut self,
        user: &SelfUser,
        message: MlsMessageIn,
    ) -> Result<ProcessedMessage, GroupOperationError> {
        Ok(self
            .mls_group
            .process_message(&user.crypto_backend, message)?)
    }

    /// Invite user with the given name to the group.
    pub(crate) fn invite<'a>(
        &'a mut self,
        user: &SelfUser,
        key_package: KeyPackage,
        backend: &Backend,
    ) -> Result<&'a StagedCommit, GroupOperationError> {
        let (mls_message_out, welcome_msg, _group_info_option) =
            self.mls_group
                .add_members(&user.crypto_backend, user.signer(), &[key_package])?;

        // Make a copy of the pending commit for later inspection
        let staged_commit = self.mls_group.pending_commit().unwrap();

        // Filter out own user and create a list of members to whom to send the
        // commit.
        let members = self
            .mls_group
            .members()
            .filter_map(|member| {
                if member.credential != user.credential_with_key.credential {
                    Some(member.credential.identity().to_vec())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Send Commit to the group.
        log::trace!("Sending Commit");

        let msg = GroupMessage::new(mls_message_out.into(), &members);
        backend
            .send_msg(&msg)
            .map_err(|_| GroupOperationError::InvitationError)?;

        // Send Welcome to the client.
        log::trace!("Sending Welcome");

        backend
            .send_welcome(&welcome_msg)
            .map_err(|_| GroupOperationError::InvitationError)?;

        Ok(staged_commit)
    }

    /// Merge the pending commit
    pub(crate) fn merge_pending_commit(
        &mut self,
        user: &SelfUser,
    ) -> Result<(), GroupOperationError> {
        Ok(self.mls_group.merge_pending_commit(&user.crypto_backend)?)
    }

    /// Get a list of clients in the group to send messages to.
    fn recipients(&self, user: &SelfUser) -> Vec<Vec<u8>> {
        let recipients: Vec<Vec<u8>> = self
            .mls_group
            .members()
            .into_iter()
            .filter_map(|kp| {
                if user.credential_with_key.credential.identity() != kp.credential.identity() {
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
        user: &SelfUser,
        msg: &str,
    ) -> Result<GroupMessage, GroupOperationError> {
        let mls_message_out =
            self.mls_group
                .create_message(&user.crypto_backend, user.signer(), msg.as_bytes())?;

        Ok(GroupMessage::new(
            mls_message_out.into(),
            &self.recipients(user),
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
