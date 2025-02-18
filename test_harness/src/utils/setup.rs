// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{HashMap, HashSet};

use phnxcoreclient::{
    clients::CoreUser, store::Store, ConversationId, ConversationStatus, ConversationType, *,
};
use phnxserver::network_provider::MockNetworkProvider;
use phnxtypes::{
    identifiers::{Fqdn, QualifiedUserName},
    DEFAULT_PORT_HTTP,
};
use rand::{distributions::Alphanumeric, seq::IteratorRandom, Rng, RngCore};
use rand_chacha::rand_core::OsRng;
use tracing::info;

use super::spawn_app;

pub struct TestUser {
    pub user: CoreUser,
}

impl AsRef<CoreUser> for TestUser {
    fn as_ref(&self) -> &CoreUser {
        &self.user
    }
}

impl AsMut<CoreUser> for TestUser {
    fn as_mut(&mut self) -> &mut CoreUser {
        &mut self.user
    }
}

impl TestUser {
    pub async fn new(user_name: &QualifiedUserName, address_option: Option<String>) -> Self {
        let hostname_str = address_option
            .unwrap_or_else(|| format!("{}:{}", user_name.domain(), DEFAULT_PORT_HTTP));

        let server_url = format!("http://{}", hostname_str);

        let user =
            CoreUser::new_ephemeral(user_name.clone(), &user_name.to_string(), server_url, None)
                .await
                .unwrap();
        Self { user }
    }

    pub async fn new_persisted(
        user_name: &QualifiedUserName,
        address_option: Option<String>,
        db_dir: &str,
    ) -> Self {
        let hostname_str = address_option
            .unwrap_or_else(|| format!("{}:{}", user_name.domain(), DEFAULT_PORT_HTTP));

        let server_url = format!("http://{}", hostname_str);

        let user = CoreUser::new(
            user_name.clone(),
            &user_name.to_string(),
            server_url,
            db_dir,
            None,
        )
        .await
        .unwrap();
        Self { user }
    }

    pub fn user(&self) -> &CoreUser {
        &self.user
    }
}

enum TestKind {
    SingleBackend(String), // url of the single backend
    Federated,
}

pub struct TestBackend {
    pub users: HashMap<QualifiedUserName, TestUser>,
    pub groups: HashMap<ConversationId, HashSet<QualifiedUserName>>,
    // This is what we feed to the test clients.
    kind: TestKind,
}

impl TestBackend {
    pub fn federated() -> Self {
        Self {
            users: HashMap::new(),
            groups: HashMap::new(),
            kind: TestKind::Federated,
        }
    }
    pub async fn single() -> Self {
        let network_provider = MockNetworkProvider::new();
        let domain: Fqdn = "example.com".parse().unwrap();
        let (address, _ws_dispatch) = spawn_app(domain.clone(), network_provider).await;
        Self {
            users: HashMap::new(),
            groups: HashMap::new(),
            kind: TestKind::SingleBackend(address.to_string()),
        }
    }

    pub fn url(&self) -> Option<String> {
        if let TestKind::SingleBackend(url) = &self.kind {
            Some(url.clone())
        } else {
            None
        }
    }

    pub async fn add_persisted_user(&mut self, user_name: &QualifiedUserName) {
        info!("Creating {user_name}");
        let user = TestUser::new_persisted(user_name, self.url(), "./").await;
        self.users.insert(user_name.clone(), user);
    }

    pub async fn add_user(&mut self, user_name: &QualifiedUserName) {
        info!("Creating {user_name}");
        let user = TestUser::new(user_name, self.url()).await;
        self.users.insert(user_name.clone(), user);
    }

    pub fn get_user(&self, user_name: &QualifiedUserName) -> &TestUser {
        self.users.get(user_name).unwrap()
    }

    /// This has the updater commit an update, but without the checks ensuring
    /// that the group state remains unchanged.
    pub async fn commit_to_proposals(
        &mut self,
        conversation_id: ConversationId,
        updater_name: QualifiedUserName,
    ) {
        info!(
            "{} performs an update in group {}",
            updater_name,
            conversation_id.uuid()
        );

        let test_updater = self.users.get_mut(&updater_name).unwrap();
        let updater = &mut test_updater.user;

        let pending_removes = HashSet::<QualifiedUserName>::from_iter(
            updater.pending_removes(conversation_id).await.unwrap(),
        );
        let group_members_before = updater
            .conversation_participants(conversation_id)
            .await
            .unwrap();

        updater.update(conversation_id).await.unwrap();

        let group_members_after = updater
            .conversation_participants(conversation_id)
            .await
            .unwrap();
        let difference: HashSet<QualifiedUserName> = group_members_before
            .difference(&group_members_after)
            .map(|s| s.to_owned())
            .collect();
        assert_eq!(difference, pending_removes);

        let group_members = self.groups.get(&conversation_id).unwrap();
        // Have all group members fetch and process messages.
        for group_member_name in group_members.iter() {
            // skip the sender
            if group_member_name == &updater_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let pending_removes = HashSet::<QualifiedUserName>::from_iter(
                group_member.pending_removes(conversation_id).await.unwrap(),
            );
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            // If the group member in question is removed with this commit,
            // it should turn its conversation inactive ...
            if pending_removes.contains(group_member_name) {
                let conversation_after = group_member.conversation(&conversation_id).await.unwrap();
                assert!(matches!(&conversation_after.status(),
                ConversationStatus::Inactive(ic)
                if HashSet::<QualifiedUserName>::from_iter(ic.past_members().to_vec()) ==
                    group_members_before
                ));
            } else {
                // ... if not, it should remove the members to be removed.
                let group_members_after = group_member
                    .conversation_participants(conversation_id)
                    .await
                    .unwrap();
                let difference: HashSet<QualifiedUserName> = group_members_before
                    .difference(&group_members_after)
                    .map(|s| s.to_owned())
                    .collect();
                assert_eq!(difference, pending_removes);
            }
        }
    }

    pub async fn update_group(
        &mut self,
        conversation_id: ConversationId,
        updater_name: &QualifiedUserName,
    ) {
        info!(
            "{} performs an update in group {}",
            updater_name,
            conversation_id.uuid()
        );

        let test_updater = self.users.get_mut(updater_name).unwrap();
        let updater = &mut test_updater.user;

        updater.update(conversation_id).await.unwrap();

        let group_members = self.groups.get(&conversation_id).unwrap();
        // Have all group members fetch and process messages.
        for group_member_name in group_members.iter() {
            // skip the sender
            if group_member_name == updater_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_members_after = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            assert_eq!(group_members_after, group_members_before);
        }
    }

    pub async fn connect_users(
        &mut self,
        user1_name: &QualifiedUserName,
        user2_name: &QualifiedUserName,
    ) -> ConversationId {
        info!("Connecting users {} and {}", user1_name, user2_name);
        let test_user1 = self.users.get_mut(user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_partial_contacts_before = user1.partial_contacts().await.unwrap();
        let user1_conversations_before = user1.conversations().await.unwrap();
        user1.add_contact(user2_name.clone()).await.unwrap();
        let mut user1_partial_contacts_after = user1.partial_contacts().await.unwrap();
        let error_msg = format!(
            "User 2 should be in the partial contacts list of user 1. List: {:?}",
            user1_partial_contacts_after,
        );
        let new_user_position = user1_partial_contacts_after
            .iter()
            .position(|c| &c.user_name == user2_name)
            .expect(&error_msg);
        // If we remove the new user, the partial contact lists should be the same.
        user1_partial_contacts_after.remove(new_user_position);
        user1_partial_contacts_before
            .into_iter()
            .zip(user1_partial_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.user_name, after.user_name);
            });
        let mut user1_conversations_after = user1.conversations().await.unwrap();
        let test_title = format!("Connection group: {} - {}", user1_name, user2_name);
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| c.attributes().title() == test_title)
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type()
                == &ConversationType::UnconfirmedConnection(user2_name.clone())
        );
        user1_conversations_before
            .into_iter()
            .zip(user1_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        let user1_conversation_id = conversation.id();

        let test_user2 = self.users.get_mut(user2_name).unwrap();
        let user2 = &mut test_user2.user;
        let user2_contacts_before = user2.contacts().await.unwrap();
        let user2_conversations_before = user2.conversations().await.unwrap();
        info!("{} fetches AS messages", user2_name);
        let as_messages = user2.as_fetch_messages().await.unwrap();
        info!("{} processes AS messages", user2_name);
        user2.fully_process_as_messages(as_messages).await.unwrap();
        // User 2 should have auto-accepted (for now at least) the connection request.
        let mut user2_contacts_after = user2.contacts().await.unwrap();
        info!("User 2 contacts after: {:?}", user2_contacts_after);
        let user2_partial_contacts_before = user2.partial_contacts().await.unwrap();
        info!(
            "User 2 partial contacts after: {:?}",
            user2_partial_contacts_before
        );
        let new_contact_position = user2_contacts_after
            .iter()
            .position(|c| &c.user_name == user1_name)
            .expect("User 1 should be in the partial contacts list of user 2");
        // If we remove the new user, the partial contact lists should be the same.
        user2_contacts_after.remove(new_contact_position);
        user2_contacts_before
            .into_iter()
            .zip(user2_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.user_name, after.user_name);
            });
        // User 2 should have created a connection group.
        let mut user2_conversations_after = user2.conversations().await.unwrap();
        info!(
            "User 2 conversations after: {:?}",
            user2_conversations_after
        );
        let new_conversation_position = user2_conversations_after
            .iter()
            .position(|c| c.attributes().title() == user1_name.to_string())
            .expect("User 2 should have created a new conversation");
        let conversation = user2_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type() == &ConversationType::Connection(user1_name.clone())
        );
        user2_conversations_before
            .into_iter()
            .zip(user2_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        let user2_conversation_id = conversation.id();

        let user2_user_name = user2.user_name().clone();
        let test_user1 = self.users.get_mut(user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_contacts_before: HashSet<_> = user1
            .contacts()
            .await
            .unwrap()
            .into_iter()
            .map(|contact| contact.user_name.clone())
            .collect();
        let user1_conversations_before = user1.conversations().await.unwrap();
        info!("{} fetches QS messages", user1_name);
        let qs_messages = user1.qs_fetch_messages().await.unwrap();
        info!("{} processes QS messages", user1_name);
        user1.fully_process_qs_messages(qs_messages).await.unwrap();

        // User 1 should have added user 2 to its contacts now and a connection
        // group should have been created.
        let user1_contacts_after: HashSet<_> = user1
            .contacts()
            .await
            .unwrap()
            .into_iter()
            .map(|contact| contact.user_name.clone())
            .collect();
        let new_user_vec: Vec<_> = user1_contacts_after
            .difference(&user1_contacts_before)
            .collect();
        assert_eq!(new_user_vec, vec![&user2_user_name]);
        // User 2 should have created a connection group.
        let mut user1_conversations_after = user1.conversations().await.unwrap();
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| c.attributes().title() == test_title)
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type() == &ConversationType::Connection(user2_name.clone())
        );
        let ids_before: HashSet<_> = user1_conversations_before.iter().map(|c| c.id()).collect();
        let ids_after: HashSet<_> = user1_conversations_after.iter().map(|c| c.id()).collect();
        assert!(ids_before.is_superset(&ids_after));
        debug_assert_eq!(user1_conversation_id, user2_conversation_id);

        let user1_unread_messages = self
            .users
            .get_mut(user1_name)
            .unwrap()
            .user
            .unread_messages_count(user1_conversation_id)
            .await;
        assert_eq!(user1_unread_messages, 0);

        // Send messages both ways to ensure it works.
        self.send_message(user1_conversation_id, user1_name, vec![user2_name])
            .await;

        let user1_unread_messages = self
            .users
            .get_mut(user1_name)
            .unwrap()
            .user
            .unread_messages_count(user1_conversation_id)
            .await;
        assert_eq!(user1_unread_messages, 0);

        self.send_message(user1_conversation_id, user2_name, vec![user1_name])
            .await;

        let user1_unread_messages = self
            .users
            .get_mut(user1_name)
            .unwrap()
            .user
            .unread_messages_count(user1_conversation_id)
            .await;
        assert_eq!(user1_unread_messages, 1);

        // Fetch the last message and mark it as read.
        let test_user1 = self.users.get_mut(user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_messages = user1.get_messages(user1_conversation_id, 1).await.unwrap();

        assert_eq!(user1_messages.len(), 1);
        let user1_unread_messages = user1.unread_messages_count(user1_conversation_id).await;
        assert_eq!(user1_unread_messages, 1);

        let last_message = user1_messages.last().unwrap();

        user1
            .mark_as_read([(user1_conversation_id, last_message.timestamp())].into_iter())
            .await
            .unwrap();

        let user1_unread_messages = user1.unread_messages_count(user1_conversation_id).await;
        assert_eq!(user1_unread_messages, 0);

        let test_user2 = self.users.get_mut(user2_name).unwrap();
        let user2 = &mut test_user2.user;
        let user2_messages = user2.get_messages(user2_conversation_id, 1).await.unwrap();

        assert_eq!(user2_messages.len(), 1);
        let last_message = user2_messages.last().unwrap();
        user2
            .mark_as_read([(user2_conversation_id, last_message.timestamp())].into_iter())
            .await
            .unwrap();

        let user2_unread_messages = user2.unread_messages_count(user2_conversation_id).await;
        assert_eq!(user2_unread_messages, 0);

        let member_set: HashSet<QualifiedUserName> =
            [user1_name.clone(), user2_name.clone()].into();
        assert_eq!(member_set.len(), 2);
        self.groups.insert(user1_conversation_id, member_set);
        user1_conversation_id
    }

    /// Sends a message from the given sender to the given recipients. Before
    /// sending a message, the sender picks up its QS messages to make sure it's
    /// up to date.
    pub async fn send_message(
        &mut self,
        conversation_id: ConversationId,
        sender_name: &QualifiedUserName,
        recipient_names: Vec<&QualifiedUserName>,
    ) -> ConversationMessageId {
        let recipient_strings = recipient_names
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        info!(
            "{} sends a message to {}",
            sender_name,
            recipient_strings.join(", ")
        );
        let message: String = OsRng
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let orig_message = MimiContent::simple_markdown_message(sender_name.domain(), message);
        let test_sender = self.users.get_mut(sender_name).unwrap();
        let sender = &mut test_sender.user;

        // Before sending a message, the sender must first fetch and process its QS messages.

        let sender_qs_messages = sender.qs_fetch_messages().await.unwrap();

        sender
            .fully_process_qs_messages(sender_qs_messages)
            .await
            .unwrap();

        let message = test_sender
            .user
            .send_message(conversation_id, orig_message.clone())
            .await
            .unwrap();
        let sender_user_name = test_sender.user.user_name().to_owned();

        assert_eq!(
            message.message(),
            &Message::Content(Box::new(ContentMessage::new(
                test_sender.user.user_name().to_string(),
                true,
                orig_message.clone()
            )))
        );

        for recipient_name in &recipient_names {
            let recipient = self.users.get_mut(recipient_name).unwrap();
            let recipient_user = &mut recipient.user;

            let recipient_qs_messages = recipient_user.qs_fetch_messages().await.unwrap();

            let messages = recipient_user
                .fully_process_qs_messages(recipient_qs_messages)
                .await
                .unwrap();

            assert_eq!(
                messages.new_messages.last().unwrap().message(),
                &Message::Content(Box::new(ContentMessage::new(
                    sender_user_name.to_string(),
                    true,
                    orig_message.clone()
                )))
            );
        }
        message.id()
    }

    pub async fn create_group(&mut self, user_name: &QualifiedUserName) -> ConversationId {
        let test_user = self.users.get_mut(user_name).unwrap();
        let user = &mut test_user.user;
        let user_conversations_before = user.conversations().await.unwrap();

        let group_name = format!("{:?}", OsRng.gen::<[u8; 32]>());
        let group_picture_bytes_option = Some(OsRng.gen::<[u8; 32]>().to_vec());
        let conversation_id = user
            .create_conversation(group_name.clone(), group_picture_bytes_option.clone())
            .await
            .unwrap();
        let mut user_conversations_after = user.conversations().await.unwrap();
        let new_conversation_position = user_conversations_after
            .iter()
            .position(|c| c.attributes().title() == group_name)
            .expect("User 1 should have created a new conversation");
        let conversation = user_conversations_after.remove(new_conversation_position);
        assert!(conversation.id() == conversation_id);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(conversation.conversation_type() == &ConversationType::Group);
        assert_eq!(conversation.attributes().title(), &group_name);
        assert_eq!(
            conversation.attributes().picture(),
            group_picture_bytes_option.as_deref()
        );
        user_conversations_before
            .into_iter()
            .zip(user_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        let member_set: HashSet<QualifiedUserName> = [user_name.clone()].into();
        assert_eq!(member_set.len(), 1);
        self.groups.insert(conversation_id, member_set);

        conversation_id
    }

    /// Has the inviter invite the invitees to the given group and has everyone
    /// send and process their messages.
    pub async fn invite_to_group(
        &mut self,
        conversation_id: ConversationId,
        inviter_name: &QualifiedUserName,
        invitee_names: Vec<&QualifiedUserName>,
    ) {
        let invitee_strings = invitee_names
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let test_inviter = self.users.get_mut(inviter_name).unwrap();
        let inviter = &mut test_inviter.user;

        // Before inviting anyone to a group, the inviter must first fetch and
        // process its QS messages.
        let qs_messages = inviter.qs_fetch_messages().await.unwrap();

        inviter
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");
        let inviter_conversation = inviter.conversation(&conversation_id).await.unwrap();

        info!(
            "{} invites {} to the group with id {}",
            inviter_name,
            invitee_strings.join(", "),
            conversation_id.uuid()
        );

        // Perform the invite operation and check that the invitees are now in the group.
        let inviter_group_members_before = inviter
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");

        let invite_messages = inviter
            .invite_users(
                conversation_id,
                &invitee_names.iter().cloned().cloned().collect::<Vec<_>>(),
            )
            .await
            .expect("Error inviting users.");

        let mut expected_messages = HashSet::new();
        for invitee_name in &invitee_names {
            let expected_message = format!(
                "{} added {} to the conversation",
                inviter_name, invitee_name,
            );
            expected_messages.insert(expected_message);
        }

        let invite_messages = display_messages_to_string_map(invite_messages);

        assert_eq!(invite_messages, expected_messages);

        let inviter_group_members_after = inviter
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");
        let new_members = inviter_group_members_after
            .difference(&inviter_group_members_before)
            .collect::<HashSet<_>>();
        let invitee_set = invitee_names.iter().copied().collect::<HashSet<_>>();
        assert_eq!(new_members, invitee_set);

        // Now that the invitation is out, have the invitees and all other group
        // members fetch and process QS messages.
        for invitee_name in &invitee_names {
            let test_invitee = self.users.get_mut(invitee_name).unwrap();
            let invitee = &mut test_invitee.user;
            let mut invitee_conversations_before = invitee.conversations().await.unwrap();

            let qs_messages = invitee.qs_fetch_messages().await.unwrap();

            invitee
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let mut invitee_conversations_after = invitee.conversations().await.unwrap();
            let conversation_uuid = conversation_id.uuid();
            let new_conversation_position = invitee_conversations_after
                .iter()
                .position(|c| c.id() == conversation_id)
                .unwrap_or_else(|| panic!("{invitee_name} should have created a new conversation titles {conversation_uuid}"));
            let conversation = invitee_conversations_after.remove(new_conversation_position);
            assert!(conversation.id() == conversation_id);
            assert!(conversation.status() == &ConversationStatus::Active);
            assert!(conversation.conversation_type() == &ConversationType::Group);
            assert_eq!(
                conversation.attributes().title(),
                inviter_conversation.attributes().title()
            );
            assert_eq!(
                conversation.attributes().picture(),
                inviter_conversation.attributes().picture()
            );
            // In case it was a re-join, we remove it from the conversation list before as well.
            if let Some(inactive_conversation_position) = invitee_conversations_before
                .iter()
                .position(|c| c.id() == conversation_id)
            {
                invitee_conversations_before.remove(inactive_conversation_position);
            }
            // Now that we've removed the new conversation, it should be the same set of conversations
            info!("Conversations_before: {:?}", invitee_conversations_before);
            info!("Conversations_after: {:?}", invitee_conversations_after);
            let different_conversations = invitee_conversations_before
                .into_iter()
                .collect::<HashSet<_>>()
                .symmetric_difference(
                    &invitee_conversations_after
                        .into_iter()
                        .collect::<HashSet<_>>(),
                )
                .count();
            assert_eq!(different_conversations, 0);
        }
        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        for group_member_name in group_members.iter() {
            // Skip the sender
            if group_member_name == inviter_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let invite_messages = group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let invite_messages = display_messages_to_string_map(invite_messages.new_messages);

            assert_eq!(invite_messages, expected_messages);

            let group_members_after = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let new_members = group_members_after
                .difference(&group_members_before)
                .collect::<HashSet<_>>();
            let invitee_set = invitee_names.iter().copied().collect::<HashSet<_>>();
            assert_eq!(new_members, invitee_set)
        }

        for invitee_name in &invitee_names {
            let unique_member = group_members.insert((*invitee_name).clone());
            assert!(unique_member);
        }

        // Now send messages to check that the group works properly. This also
        // ensures that everyone involved has picked up their messages from the
        // QS and that notifications are flushed.
        self.send_message(conversation_id, inviter_name, invitee_names.clone())
            .await;
        for invitee_name in &invitee_names {
            let recipients: Vec<_> = invitee_names
                .iter()
                .filter(|&name| name != invitee_name)
                .chain([&inviter_name].into_iter())
                .map(|name| name.to_owned())
                .collect();
            self.send_message(conversation_id, invitee_name, recipients)
                .await;
        }
    }

    /// Has the remover remove the removed from the given group and has everyone
    /// send and process their messages.
    pub async fn remove_from_group(
        &mut self,
        conversation_id: ConversationId,
        remover_name: &QualifiedUserName,
        removed_names: Vec<&QualifiedUserName>,
    ) {
        let removed_strings = removed_names
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let test_remover = self.users.get_mut(remover_name).unwrap();
        let remover = &mut test_remover.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = remover.qs_fetch_messages().await.unwrap();

        remover
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        info!(
            "{} removes {} from the group with id {}",
            remover_name,
            removed_strings.join(", "),
            conversation_id.uuid()
        );

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let remover_group_members_before = remover
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");

        let remove_messages = remover
            .remove_users(
                conversation_id,
                &removed_names.iter().copied().cloned().collect::<Vec<_>>(),
            )
            .await
            .expect("Error removing users.");

        let mut expected_messages = HashSet::new();

        for removed_name in &removed_names {
            let expected_message = format!(
                "{} removed {} from the conversation",
                remover_name, removed_name,
            );
            expected_messages.insert(expected_message);
        }

        let remove_messages = display_messages_to_string_map(remove_messages);
        assert_eq!(remove_messages, expected_messages);

        let remover_group_members_after = remover
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");
        let removed_members = remover_group_members_before
            .difference(&remover_group_members_after)
            .map(|name| name.to_owned())
            .collect::<HashSet<_>>();
        let removed_set = removed_names
            .iter()
            .map(|name| (*name).clone())
            .collect::<HashSet<_>>();
        assert_eq!(removed_members, removed_set);

        for removed_name in &removed_names {
            let test_removed = self.users.get_mut(removed_name).unwrap();
            let removed = &mut test_removed.user;
            let removed_conversations_before = removed
                .conversations()
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();
            let past_members = removed
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            let qs_messages = removed.qs_fetch_messages().await.unwrap();

            removed
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let removed_conversations_after = removed
                .conversations()
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();
            let conversation = removed_conversations_after
                .iter()
                .find(|c| c.id() == conversation_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{removed_name} should have the conversation with id {}",
                        conversation_id.uuid()
                    )
                });
            assert!(conversation.id() == conversation_id);
            if let ConversationStatus::Inactive(inactive_status) = &conversation.status() {
                let inactive_status_members = HashSet::<QualifiedUserName>::from_iter(
                    inactive_status.past_members().to_vec(),
                );
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
            assert!(conversation.conversation_type() == &ConversationType::Group);
            for conversation in removed_conversations_after {
                assert!(removed_conversations_before
                    .iter()
                    .any(|c| c.id() == conversation.id()))
            }
        }
        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        for removed_name in &removed_names {
            let remove_successful = group_members.remove(removed_name);
            assert!(remove_successful);
        }
        // Now have the rest of the group pick up and process their messages.
        for group_member_name in group_members.iter() {
            // Skip the remover
            if group_member_name == remover_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let remove_messages = group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let remove_messages = display_messages_to_string_map(remove_messages.new_messages);
            assert_eq!(remove_messages, expected_messages);

            let group_members_after = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let removed_members = group_members_before
                .difference(&group_members_after)
                .map(|name| name.to_owned())
                .collect::<HashSet<_>>();
            let removed_set = removed_names
                .iter()
                .map(|name| (*name).clone())
                .collect::<HashSet<_>>();
            assert_eq!(removed_members, removed_set)
        }
    }

    /// Has the leaver leave the given group.
    pub async fn leave_group(
        &mut self,
        conversation_id: ConversationId,
        leaver_name: &QualifiedUserName,
    ) {
        info!(
            "{} leaves the group with id {}",
            leaver_name,
            conversation_id.uuid()
        );
        let test_leaver = self.users.get_mut(leaver_name).unwrap();
        let leaver = &mut test_leaver.user;

        // Perform the leave operation.
        leaver.leave_conversation(conversation_id).await.unwrap();

        // Now have a random group member perform an update, thus committing the leave operation.
        // TODO: This is not really random. We should do better here. But also,
        // we probably want a way to track the randomness s.t. we can reproduce
        // tests.
        let group_members = self.groups.get(&conversation_id).unwrap().clone();
        let mut random_member_iter = group_members.iter();
        let mut random_member_name = random_member_iter.next().unwrap();
        // Ensure that the random member isn't the leaver.
        if random_member_name == leaver_name {
            random_member_name = random_member_iter.next().unwrap()
        }
        let test_random_member = self.users.get_mut(random_member_name).unwrap();
        let random_member = &mut test_random_member.user;

        // First fetch and process the QS messages to make sure the member has the proposal.
        let qs_messages = random_member.qs_fetch_messages().await.unwrap();

        random_member
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        // Now commit to the pending proposal. This also makes everyone else
        // pick up and process their messages. This also tests that group
        // members were removed correctly from the local group and that the
        // leaver has turned its conversation inactive.
        self.commit_to_proposals(conversation_id, random_member_name.clone())
            .await;

        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        group_members.remove(leaver_name);
    }

    pub async fn delete_group(
        &mut self,
        conversation_id: ConversationId,
        deleter_name: &QualifiedUserName,
    ) {
        info!(
            "{} deletes the group with id {}",
            deleter_name,
            conversation_id.uuid()
        );
        let test_deleter = self.users.get_mut(deleter_name).unwrap();
        let deleter = &mut test_deleter.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = deleter.qs_fetch_messages().await.unwrap();

        deleter
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let deleter_conversation_before = deleter
            .conversation(&conversation_id)
            .await
            .unwrap()
            .clone();
        assert_eq!(
            deleter_conversation_before.status(),
            &ConversationStatus::Active
        );
        let past_members = deleter
            .conversation_participants(conversation_id)
            .await
            .unwrap();

        deleter.delete_conversation(conversation_id).await.unwrap();

        let deleter_conversation_after = deleter.conversation(&conversation_id).await.unwrap();
        if let ConversationStatus::Inactive(inactive_status) = &deleter_conversation_after.status()
        {
            let inactive_status_members =
                HashSet::<QualifiedUserName>::from_iter(inactive_status.past_members().to_vec());
            assert_eq!(inactive_status_members, past_members);
        } else {
            panic!("Conversation should be inactive.")
        }

        for group_member_name in self.groups.get(&conversation_id).unwrap().iter() {
            // Skip the deleter
            if group_member_name == deleter_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;

            let group_member_conversation_before =
                group_member.conversation(&conversation_id).await.unwrap();
            assert_eq!(
                group_member_conversation_before.status(),
                &ConversationStatus::Active
            );
            let past_members = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_member_conversation_after =
                group_member.conversation(&conversation_id).await.unwrap();
            if let ConversationStatus::Inactive(inactive_status) =
                &group_member_conversation_after.status()
            {
                let inactive_status_members = HashSet::<QualifiedUserName>::from_iter(
                    inactive_status.past_members().to_vec(),
                );
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
        }
        self.groups.remove(&conversation_id);
    }

    pub fn random_user(&self, rng: &mut impl RngCore) -> QualifiedUserName {
        self.users
            .keys()
            .choose(rng)
            .expect("There should be at least one user")
            .clone()
    }

    pub async fn perform_random_operation(&mut self, rng: &mut impl RngCore) {
        // Get a user to perform the operation
        let random_user = self.random_user(rng);
        // Possible actions:
        // 0: Establish a connection
        // 1: Create a group and invite one or more users
        // 2: Invite up to 5 users to a group
        // 3: Remove up to 5 users from a group
        // 4: Leave a group
        // Message sending is covered, as it's done as part of all of those
        // actions. If one of the actions is not possible, it is skipped.
        // TODO: Breaking up of connections
        let action = rng.gen_range(0..=3);
        match action {
            // Establish a connection
            0 => {
                let mut other_users = Vec::new();
                for user in self.users.keys() {
                    let is_contact = self
                        .users
                        .get(user)
                        .unwrap()
                        .user()
                        .contacts()
                        .await
                        .unwrap()
                        .into_iter()
                        .any(|contact| contact.user_name == random_user);
                    if user != &random_user && !is_contact {
                        other_users.push(user.clone());
                    }
                }
                if let Some(other_user) = other_users.into_iter().choose(rng) {
                    info!(
                        random_operation = true,
                        "Random operation: Connecting {} and {}", random_user, other_user
                    );
                    self.connect_users(&random_user, &other_user).await;
                }
            }
            1 => {
                let conversation_id = self.create_group(&random_user).await;
                info!(
                    random_operation = true,
                    "Random operation: Created group {}",
                    conversation_id.uuid()
                );
                // TODO: Invite user(s)
            }
            2 => {
                // Pick a group
                let user = self.users.get(&random_user).unwrap();
                // Let's exclude connection groups for now.
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
                    })
                    .choose(rng)
                {
                    let number_of_invitees = rng.gen_range(1..=5);
                    let mut invitee_names = Vec::new();
                    for invitee in self.users.keys() {
                        let is_group_member = self
                            .groups
                            .get(&conversation.id())
                            .unwrap()
                            .contains(invitee);
                        let is_connected = user
                            .user()
                            .contacts()
                            .await
                            .unwrap()
                            .into_iter()
                            .any(|contact| &contact.user_name == invitee);
                        if !is_group_member && is_connected && invitee != &random_user {
                            invitee_names.push(invitee);
                        }
                    }
                    let invitee_names = invitee_names
                        .into_iter()
                        .cloned()
                        .choose_multiple(rng, number_of_invitees);
                    // It can happen that there are no suitable users to invite
                    if !invitee_names.is_empty() {
                        let invitee_strings = invitee_names
                            .iter()
                            .map(|invitee| invitee.to_string())
                            .collect::<Vec<_>>();
                        info!(
                            random_operation = true,
                            "Random operation: {} invites {} to group {}",
                            random_user,
                            invitee_strings.join(", "),
                            conversation.id().uuid()
                        );
                        self.invite_to_group(
                            conversation.id(),
                            &random_user,
                            invitee_names.iter().collect(),
                        )
                        .await;
                    }
                }
            }
            3 => {
                let user = self.users.get(&random_user).unwrap();
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
                    })
                    .choose(rng)
                {
                    let number_of_removals = rng.gen_range(1..=5);
                    let members_to_remove = self
                        .groups
                        .get(&conversation.id())
                        .unwrap()
                        .iter()
                        .filter(|&member| member != &random_user)
                        .cloned()
                        .choose_multiple(rng, number_of_removals);
                    if !members_to_remove.is_empty() {
                        let removed_strings = members_to_remove
                            .iter()
                            .map(|removed| removed.to_string())
                            .collect::<Vec<_>>();
                        info!(
                            random_operation = true,
                            "Random operation: {} removes {} from group {}",
                            random_user,
                            removed_strings.join(", "),
                            conversation.id().uuid()
                        );
                        let members_to_remove = members_to_remove.iter().collect();
                        self.remove_from_group(conversation.id(), &random_user, members_to_remove)
                            .await;
                    }
                }
            }
            4 => {
                let user = self.users.get(&random_user).unwrap();
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
                    })
                    .choose(rng)
                {
                    info!(
                        random_operation = true,
                        "Random operation: {} leaves group {}",
                        random_user,
                        conversation.id().uuid()
                    );
                    self.leave_group(conversation.id(), &random_user).await;
                }
            }
            _ => panic!("Invalid action"),
        }
    }
}

fn display_messages_to_string_map(display_messages: Vec<ConversationMessage>) -> HashSet<String> {
    display_messages
        .into_iter()
        .filter_map(|m| {
            if let Message::Event(EventMessage::System(system_message)) = m.message() {
                Some(system_message.to_string())
            } else {
                None
            }
        })
        .collect()
}
