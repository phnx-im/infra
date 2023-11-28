// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use phnxcoreclient::{
    notifications::{Notifiable, NotificationHub},
    users::SelfUser,
    ConversationId, ConversationStatus, ConversationType, NotificationType, *,
};
use phnxserver::network_provider::MockNetworkProvider;
use phnxtypes::{
    identifiers::{Fqdn, UserName},
    DEFAULT_PORT_HTTP,
};
use rand::{seq::IteratorRandom, Rng, RngCore};
use rand_chacha::rand_core::OsRng;
<<<<<<< HEAD
use uuid::Uuid;
=======

use super::spawn_app;
>>>>>>> main

use super::spawn_app;

#[derive(Clone)]
pub struct TestNotifier {
    notifications: Arc<Mutex<Vec<NotificationType>>>,
}

impl Notifiable for TestNotifier {
    fn notify(&self, notification_type: NotificationType) -> bool {
        let mut inner = self.notifications.lock().unwrap();
        inner.push(notification_type);
        true
    }
}

impl TestNotifier {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn notifications(&mut self) -> Vec<NotificationType> {
        let mut notifications_lock = self.notifications.lock().unwrap();
        let notifications = notifications_lock.drain(..).collect();
        notifications
    }
}

pub struct TestUser {
    pub user: SelfUser<TestNotifier>,
    pub notifier: TestNotifier,
}

impl TestUser {
    pub async fn new(user_name: &UserName, address_option: Option<String>) -> Self {
        let mut notification_hub = NotificationHub::<TestNotifier>::default();
        let hostname_str = address_option
            .unwrap_or_else(|| format!("{}:{}", user_name.domain().to_string(), DEFAULT_PORT_HTTP));

        let server_url = format!("http://{}", hostname_str);

        let notifier = TestNotifier::new();
        notification_hub.add_sink(notifier.notifier());
        let user = SelfUser::new_ephemeral(
            user_name.clone(),
            &user_name.to_string(),
            server_url,
            notification_hub,
        )
        .await
        .unwrap();
        Self { user, notifier }
    }

    pub fn user(&self) -> &SelfUser<TestNotifier> {
        &self.user
    }
}

enum TestKind {
    SingleBackend(String), // url of the single backend
    Federated,
}

pub struct TestBackend {
    pub users: HashMap<UserName, TestUser>,
<<<<<<< HEAD
    pub groups: HashMap<Uuid, HashSet<UserName>>,
=======
    pub groups: HashMap<ConversationId, HashSet<UserName>>,
>>>>>>> main
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
        let domain = Fqdn::from("example.com");
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

    pub async fn add_user(&mut self, user_name: impl Into<UserName>) {
        let user_name = user_name.into();
        tracing::info!("Creating {user_name}");
        let user = TestUser::new(&user_name, self.url()).await;
        self.users.insert(user_name, user);
    }

    pub fn flush_notifications(&mut self) {
        self.users.values_mut().for_each(|u| {
            let _ = u.notifier.notifications();
        });
    }

    /// This has the updater commit an update, but without the checks ensuring
    /// that the group state remains unchanged.
    pub async fn commit_to_proposals(
        &mut self,
        conversation_id: ConversationId,
        updater_name: impl Into<UserName>,
    ) {
        let updater_name = &updater_name.into();
        tracing::info!(
            "{} performs an update in group {}",
            updater_name,
            conversation_id.as_uuid()
        );

        let test_updater = self.users.get_mut(&updater_name).unwrap();
        let updater = &mut test_updater.user;

        let pending_removes =
            HashSet::<UserName>::from_iter(updater.pending_removes(conversation_id).unwrap());
        let group_members_before = updater.group_members(conversation_id).unwrap();

        updater.update(conversation_id).await.unwrap();

        let group_members_after =
            HashSet::<UserName>::from_iter(updater.group_members(conversation_id).unwrap());
        let difference: HashSet<UserName> = HashSet::<UserName>::from_iter(group_members_before)
            .difference(&group_members_after)
            .map(|s| s.to_owned())
            .collect();
        assert_eq!(difference, pending_removes);

        let group_members = self.groups.get(&conversation_id).unwrap();
        // Have all group members fetch and process messages.
        for group_member_name in group_members.iter() {
            // skip the sender
            if group_member_name == updater_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let pending_removes = HashSet::<UserName>::from_iter(
                group_member.pending_removes(conversation_id).unwrap(),
            );
            let group_members_before = group_member.group_members(conversation_id).unwrap();

            group_member
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            // If the group member in question is removed with this commit,
            // it should turn its conversation inactive ...
            if pending_removes.contains(group_member_name) {
                let conversation_after = group_member.conversation(conversation_id).unwrap();
<<<<<<< HEAD
                assert!(matches!(&conversation_after.status,
                ConversationStatus::Inactive(ic)
                if HashSet::<UserName>::from_iter(ic.past_members()) ==
=======
                assert!(matches!(&conversation_after.status(),
                ConversationStatus::Inactive(ic)
                if HashSet::<UserName>::from_iter(ic.past_members().to_vec()) ==
>>>>>>> main
                    HashSet::<UserName>::from_iter(group_members_before)
                ));
            } else {
                // ... if not, it should remove the members to be removed.
                let group_members_after = HashSet::<UserName>::from_iter(
                    group_member.group_members(conversation_id).unwrap(),
                );
                let difference: HashSet<UserName> =
                    HashSet::<UserName>::from_iter(group_members_before)
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
        updater_name: impl Into<UserName>,
    ) {
        let updater_name = &updater_name.into();
        tracing::info!(
            "{} performs an update in group {}",
            updater_name,
            conversation_id.as_uuid()
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
            let group_members_before = HashSet::<UserName>::from_iter(
                group_member.group_members(conversation_id).unwrap(),
            );

            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_members_after = HashSet::<UserName>::from_iter(
                group_member.group_members(conversation_id).unwrap(),
            );
            assert_eq!(group_members_after, group_members_before);
        }
    }

    pub async fn connect_users(
        &mut self,
        user1_name: impl Into<UserName>,
        user2_name: impl Into<UserName>,
    ) -> ConversationId {
        let user1_name = user1_name.into();
        let user2_name = user2_name.into();
        tracing::info!("Connecting users {} and {}", user1_name, user2_name);
        let test_user1 = self.users.get_mut(&user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_partial_contacts_before = user1.partial_contacts().unwrap();
        let user1_conversations_before = user1.conversations().unwrap();
        user1.add_contact(user2_name.clone()).await.unwrap();
        let mut user1_partial_contacts_after = user1.partial_contacts().unwrap();
        let error_msg = format!(
            "User 2 should be in the partial contacts list of user 1. List: {:?}",
            user1_partial_contacts_after,
        );
        let new_user_position = user1_partial_contacts_after
            .iter()
            .position(|c| c.user_name == user2_name)
            .expect(&error_msg);
        // If we remove the new user, the partial contact lists should be the same.
        user1_partial_contacts_after.remove(new_user_position);
        user1_partial_contacts_before
            .into_iter()
            .zip(user1_partial_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.user_name, after.user_name);
            });
        let mut user1_conversations_after = user1.conversations().unwrap();
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| c.attributes().title == user2_name.to_string())
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
        let user1_conversation_id = conversation.id().clone();

        let test_user2 = self.users.get_mut(&user2_name).unwrap();
        let user2 = &mut test_user2.user;
        let user2_contacts_before = user2.contacts().unwrap();
        let user2_conversations_before = user2.conversations().unwrap();
        tracing::info!("{} fetches AS messages", user2_name);
        let as_messages = user2.as_fetch_messages().await.unwrap();
        tracing::info!("{} processes AS messages", user2_name);
        user2.process_as_messages(as_messages).await.unwrap();
        // User 2 should have auto-accepted (for now at least) the connection request.
        let mut user2_contacts_after = user2.contacts().unwrap();
        let new_contact_position = user2_contacts_after
            .iter()
            .position(|c| c.user_name == user1_name)
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
        let mut user2_conversations_after = user2.conversations().unwrap();
        let new_conversation_position = user2_conversations_after
            .iter()
            .position(|c| c.attributes().title == user1_name.to_string())
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
        let test_user1 = self.users.get_mut(&user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_contacts_before: HashSet<_> = user1
            .contacts()
            .unwrap()
            .into_iter()
            .map(|contact| contact.user_name.clone())
            .collect();
        let user1_conversations_before = user1.conversations().unwrap();
        tracing::info!("{} fetches QS messages", user1_name);
        let qs_messages = user1.qs_fetch_messages().await.unwrap();
        tracing::info!("{} processes QS messages", user1_name);
        user1.process_qs_messages(qs_messages).await.unwrap();

        // User 1 should have added user 2 to its contacts now and a connection
        // group should have been created.
        let user1_contacts_after: HashSet<_> = user1
            .contacts()
            .unwrap()
            .into_iter()
            .map(|contact| contact.user_name.clone())
            .collect();
        let new_user_vec: Vec<_> = user1_contacts_after
            .difference(&user1_contacts_before)
            .collect();
        assert_eq!(new_user_vec, vec![&user2_user_name]);
        // User 2 should have created a connection group.
        let mut user1_conversations_after = user1.conversations().unwrap();
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| &c.attributes().title == &user2_name.to_string())
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type() == &ConversationType::Connection(user2_name.clone())
        );
        user1_conversations_before
            .into_iter()
            .zip(user1_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        self.flush_notifications();
        debug_assert_eq!(user1_conversation_id, user2_conversation_id);

        // Send messages both ways to ensure it works.
        self.send_message(
            user1_conversation_id,
            user1_name.clone(),
            vec![user2_name.clone()],
        )
        .await;
        self.send_message(
            user1_conversation_id,
            user2_name.clone(),
            vec![user1_name.clone()],
        )
        .await;

        let member_set: HashSet<UserName> = [user1_name, user2_name].into();
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
        sender_name: impl Into<UserName>,
        recipient_names: Vec<impl Into<UserName>>,
    ) {
        let sender_name = sender_name.into();
        let recipient_names: Vec<UserName> = recipient_names
            .into_iter()
            .map(|name| name.into())
            .collect::<Vec<_>>();
        let recipient_strings = recipient_names
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        tracing::info!(
            "{} sends a message to {}",
            sender_name,
            recipient_strings.join(", ")
        );
        let message: Vec<u8> = OsRng.gen::<[u8; 32]>().to_vec();
        let orig_message = MessageContentType::Text(phnxcoreclient::TextMessage::new(message));
        let test_sender = self.users.get_mut(&sender_name).unwrap();
        let sender = &mut test_sender.user;

        // Before sending a message, the sender must first fetch and process its QS messages.

        let sender_qs_messages = sender.qs_fetch_messages().await.unwrap();

        sender
            .process_qs_messages(sender_qs_messages)
            .await
            .unwrap();

        let message = test_sender
            .user
            .send_message(conversation_id, orig_message.clone())
            .await
            .unwrap();
        let sender_user_name = test_sender.user.user_name().to_owned();

        assert_eq!(
            message.message,
            Message::Content(ContentMessage {
                sender: test_sender.user.user_name().to_string(),
                content: orig_message.clone()
            })
        );

        for recipient_name in &recipient_names {
            let recipient = self.users.get_mut(recipient_name).unwrap();
            let recipient_user = &mut recipient.user;
            // Flush notifications
            //let _recipient_notifications = recipient.notifier.notifications();
            let recipient_qs_messages = recipient_user.qs_fetch_messages().await.unwrap();

            recipient_user
                .process_qs_messages(recipient_qs_messages)
                .await
                .unwrap();

            let recipient_notifications = recipient.notifier.notifications();

            assert!(matches!(
                recipient_notifications.last().unwrap(),
                NotificationType::Message(_)
            ));

            if let NotificationType::Message(message) = &recipient_notifications.last().unwrap() {
                assert_eq!(
                    message.conversation_message.message,
                    Message::Content(ContentMessage {
                        sender: sender_user_name.to_string(),
                        content: orig_message.clone()
                    })
                );
            }
        }
        self.flush_notifications();
    }

    pub async fn create_group(&mut self, user_name: impl Into<UserName>) -> ConversationId {
        let user_name = user_name.into();
        let test_user = self.users.get_mut(&user_name).unwrap();
        let user = &mut test_user.user;
        let user_conversations_before = user.conversations().unwrap();

        let group_name = format!("{:?}", OsRng.gen::<[u8; 32]>());
        let conversation_id = user.create_conversation(&group_name).await.unwrap();
        let mut user_conversations_after = user.conversations().unwrap();
        let new_conversation_position = user_conversations_after
            .iter()
            .position(|c| c.attributes().title == group_name)
            .expect("User 1 should have created a new conversation");
        let conversation = user_conversations_after.remove(new_conversation_position);
        assert!(conversation.id() == conversation_id);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(conversation.conversation_type() == &ConversationType::Group);
        user_conversations_before
            .into_iter()
            .zip(user_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        self.flush_notifications();
        let member_set: HashSet<UserName> = [user_name].into();
        assert_eq!(member_set.len(), 1);
        self.groups.insert(conversation_id, member_set);

        conversation_id
    }

    /// Has the inviter invite the invitees to the given group and has everyone
    /// send and process their messages.
    pub async fn invite_to_group(
        &mut self,
        conversation_id: ConversationId,
        inviter_name: impl Into<UserName>,
        invitee_names: Vec<impl Into<UserName>>,
    ) {
        let inviter_name = inviter_name.into();
        let invitee_names: Vec<UserName> = invitee_names
            .into_iter()
            .map(|name| name.into())
            .collect::<Vec<_>>();
        let invitee_strings = invitee_names
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let test_inviter = self.users.get_mut(&inviter_name).unwrap();
        let inviter = &mut test_inviter.user;

        // Before inviting anyone to a group, the inviter must first fetch and
        // process its QS messages.
        let qs_messages = inviter.qs_fetch_messages().await.unwrap();

        inviter
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        tracing::info!(
            "{} invites {} to the group with id {}",
            inviter_name,
            invitee_strings.join(", "),
            conversation_id.as_uuid()
        );

        // Perform the invite operation and check that the invitees are now in the group.
        let inviter_group_members_before = HashSet::<UserName>::from_iter(
            inviter
                .group_members(conversation_id)
                .expect("Error getting group members."),
        );

        inviter
            .invite_users(conversation_id, &invitee_names)
            .await
            .expect("Error inviting users.");

        let inviter_group_members_after = HashSet::<UserName>::from_iter(
            inviter
                .group_members(conversation_id)
                .expect("Error getting group members."),
        );
        let new_members = inviter_group_members_after
            .difference(&inviter_group_members_before)
            .collect::<HashSet<_>>();
        let invitee_set = invitee_names.iter().collect::<HashSet<_>>();
        assert_eq!(new_members, invitee_set);

        // Now that the invitation is out, have the invitees and all other group
        // members fetch and process QS messages.
        for invitee_name in &invitee_names {
            let test_invitee = self.users.get_mut(invitee_name).unwrap();
            let invitee = &mut test_invitee.user;
<<<<<<< HEAD
            let invitee_conversations_before = invitee.conversations().unwrap();
=======
            let mut invitee_conversations_before = invitee.conversations().unwrap();
>>>>>>> main

            let qs_messages = invitee.qs_fetch_messages().await.unwrap();

            invitee
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let mut invitee_conversations_after = invitee.conversations().unwrap();
<<<<<<< HEAD
            let new_conversation_position = invitee_conversations_after
                .iter()
                .position(|c| c.id.as_uuid() == conversation_id)
                .expect(&format!("{invitee_name} should have created a new conversation titles {conversation_id}"));
            let conversation = invitee_conversations_after.remove(new_conversation_position);
            assert!(conversation.id.as_uuid() == conversation_id);
            assert!(conversation.status == ConversationStatus::Active);
            assert!(conversation.conversation_type == ConversationType::Group);
            invitee_conversations_before
                .into_iter()
                .zip(invitee_conversations_after)
                .for_each(|(before, after)| {
                    assert_eq!(before.id, after.id);
                });
=======
            let conversation_uuid = conversation_id.as_uuid();
            let new_conversation_position = invitee_conversations_after
                .iter()
                .position(|c| c.id() == conversation_id)
                .expect(&format!("{invitee_name} should have created a new conversation titles {conversation_uuid}"));
            let conversation = invitee_conversations_after.remove(new_conversation_position);
            assert!(conversation.id() == conversation_id);
            assert!(conversation.status() == &ConversationStatus::Active);
            assert!(conversation.conversation_type() == &ConversationType::Group);
            // In case it was a re-join, we remove it from the conversation list before as well.
            if let Some(inactive_conversation_position) = invitee_conversations_before
                .iter()
                .position(|c| c.id() == conversation_id)
            {
                invitee_conversations_before.remove(inactive_conversation_position);
            }
            // Now that we've removed the new conversation, it should be the same set of conversations
            tracing::info!("Conversations_before: {:?}", invitee_conversations_before);
            tracing::info!("Conversations_after: {:?}", invitee_conversations_after);
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
>>>>>>> main
        }
        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        for group_member_name in group_members.iter() {
            // Skip the sender
            if group_member_name == &inviter_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = HashSet::<UserName>::from_iter(
                group_member.group_members(conversation_id).unwrap(),
            );
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_members_after = HashSet::<UserName>::from_iter(
                group_member.group_members(conversation_id).unwrap(),
            );
            let new_members = group_members_after
                .difference(&group_members_before)
                .collect::<HashSet<_>>();
            let invitee_set = invitee_names.iter().collect::<HashSet<_>>();
            assert_eq!(new_members, invitee_set)
        }
        for invitee_name in &invitee_names {
            let unique_member = group_members.insert(invitee_name.clone());
            assert!(unique_member == true);
        }
        self.flush_notifications();
        // Now send messages to check that the group works properly. This also
        // ensures that everyone involved has picked up their messages from the
        // QS and that notifications are flushed.
        self.send_message(conversation_id, inviter_name.clone(), invitee_names.clone())
            .await;
        for invitee_name in &invitee_names {
            let recipients: Vec<_> = invitee_names
                .iter()
                .filter(|&name| name != invitee_name)
                .chain([&inviter_name].into_iter())
                .map(|name| name.to_owned())
                .collect();
            self.send_message(conversation_id, invitee_name.clone(), recipients)
                .await;
        }
    }

    /// Has the remover remove the removed from the given group and has everyone
    /// send and process their messages.
    pub async fn remove_from_group(
        &mut self,
        conversation_id: ConversationId,
        remover_name: impl Into<UserName>,
        removed_names: Vec<impl Into<UserName>>,
    ) {
        let remover_name = remover_name.into();
        let removed_names: Vec<UserName> = removed_names
            .into_iter()
            .map(|name| name.into())
            .collect::<Vec<_>>();
        let removed_strings = removed_names
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        let test_remover = self.users.get_mut(&remover_name).unwrap();
        let remover = &mut test_remover.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = remover.qs_fetch_messages().await.unwrap();

        remover
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        tracing::info!(
            "{} removes {} from the group with id {}",
            remover_name,
            removed_strings.join(", "),
            conversation_id.as_uuid()
        );

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let remover_group_members_before = remover
            .group_members(conversation_id)
            .expect("Error getting group members.");

        remover
            .remove_users(conversation_id, &removed_names)
            .await
            .expect("Error removing users.");

        let remover_group_members_after = HashSet::<UserName>::from_iter(
            remover
                .group_members(conversation_id)
                .expect("Error getting group members."),
        );
        let removed_members = HashSet::<UserName>::from_iter(remover_group_members_before)
            .difference(&remover_group_members_after)
            .map(|name| name.to_owned())
            .collect::<HashSet<_>>();
        let removed_set = removed_names
            .iter()
            .map(|name| name.to_owned())
            .collect::<HashSet<_>>();
        assert_eq!(removed_members, removed_set);

        for removed_name in &removed_names {
            let test_removed = self.users.get_mut(removed_name).unwrap();
            let removed = &mut test_removed.user;
            let removed_conversations_before = removed
                .conversations()
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();
            let past_members =
                HashSet::<UserName>::from_iter(removed.group_members(conversation_id).unwrap());

            let qs_messages = removed.qs_fetch_messages().await.unwrap();

            removed
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let removed_conversations_after = removed
                .conversations()
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();
            let conversation = removed_conversations_after
                .iter()
                .find(|c| c.id() == conversation_id)
                .expect(&format!(
                    "{removed_name} should have the conversation with id {}",
                    conversation_id.as_uuid()
                ));
            assert!(conversation.id() == conversation_id);
            if let ConversationStatus::Inactive(inactive_status) = &conversation.status() {
                let inactive_status_members =
                    HashSet::<UserName>::from_iter(inactive_status.past_members().to_vec());
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
<<<<<<< HEAD
            assert!(conversation.conversation_type == ConversationType::Group);
            for conversation in removed_conversations_after {
                assert!(removed_conversations_before
                    .iter()
                    .any(|c| c.id == conversation.id))
=======
            assert!(conversation.conversation_type() == &ConversationType::Group);
            for conversation in removed_conversations_after {
                assert!(removed_conversations_before
                    .iter()
                    .any(|c| c.id() == conversation.id()))
>>>>>>> main
            }
        }
        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        for removed_name in &removed_names {
            let remove_successful = group_members.remove(removed_name);
            assert!(remove_successful == true);
        }
        // Now have the rest of the group pick up and process their messages.
        for group_member_name in group_members.iter() {
            // Skip the remover
            if group_member_name == &remover_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member.group_members(conversation_id).unwrap();
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_members_after = HashSet::<UserName>::from_iter(
                group_member.group_members(conversation_id).unwrap(),
            );
            let removed_members = HashSet::<UserName>::from_iter(group_members_before)
                .difference(&group_members_after)
                .map(|name| name.to_owned())
                .collect::<HashSet<_>>();
            let removed_set = removed_names
                .iter()
                .map(|name| name.to_owned())
                .collect::<HashSet<_>>();
            assert_eq!(removed_members, removed_set)
        }

        self.flush_notifications();
    }

    /// Has the leaver leave the given group.
    pub async fn leave_group(
        &mut self,
        conversation_id: ConversationId,
        leaver_name: impl Into<UserName>,
    ) {
        let leaver_name = leaver_name.into();
        tracing::info!(
            "{} leaves the group with id {}",
            leaver_name,
            conversation_id.as_uuid()
        );
        let test_leaver = self.users.get_mut(&leaver_name).unwrap();
        let leaver = &mut test_leaver.user;

        // Perform the leave operation.
        leaver.leave_group(conversation_id).await.unwrap();

        // Now have a random group member perform an update, thus committing the leave operation.
        // TODO: This is not really random. We should do better here. But also,
        // we probably want a way to track the randomness s.t. we can reproduce
        // tests.
        let group_members = self.groups.get(&conversation_id).unwrap().clone();
        let mut random_member_iter = group_members.iter();
        let mut random_member_name = random_member_iter.next().unwrap();
        // Ensure that the random member isn't the leaver.
        if random_member_name == &leaver_name {
            random_member_name = random_member_iter.next().unwrap()
        }
        let test_random_member = self.users.get_mut(random_member_name).unwrap();
        let random_member = &mut test_random_member.user;

        // First fetch and process the QS messages to make sure the member has the proposal.
        let qs_messages = random_member.qs_fetch_messages().await.unwrap();

        random_member
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        // Now commit to the pending proposal. This also makes everyone else
        // pick up and process their messages. This also tests that group
        // members were removed correctly from the local group and that the
        // leaver has turned its conversation inactive.
        self.commit_to_proposals(conversation_id, random_member_name.clone())
            .await;

        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        group_members.remove(&leaver_name);

        self.flush_notifications();
    }

    pub async fn delete_group(
        &mut self,
        conversation_id: ConversationId,
        deleter_name: impl Into<UserName>,
    ) {
        let deleter_name = deleter_name.into();
        tracing::info!(
            "{} deletes the group with id {}",
            deleter_name,
            conversation_id.as_uuid()
        );
        let test_deleter = self.users.get_mut(&deleter_name).unwrap();
        let deleter = &mut test_deleter.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = deleter.qs_fetch_messages().await.unwrap();

        deleter
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let deleter_conversation_before = deleter.conversation(conversation_id).unwrap().clone();
        assert_eq!(
            deleter_conversation_before.status(),
            &ConversationStatus::Active
        );
        let past_members =
            HashSet::<UserName>::from_iter(deleter.group_members(conversation_id).unwrap());

        deleter.delete_group(conversation_id).await.unwrap();

        let deleter_conversation_after = deleter.conversation(conversation_id).unwrap();
        if let ConversationStatus::Inactive(inactive_status) = &deleter_conversation_after.status()
        {
            let inactive_status_members =
                HashSet::<UserName>::from_iter(inactive_status.past_members().to_vec());
            assert_eq!(inactive_status_members, past_members);
        } else {
            panic!("Conversation should be inactive.")
        }

        for group_member_name in self.groups.get(&conversation_id).unwrap().iter() {
            // Skip the deleter
            if group_member_name == &deleter_name {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_name).unwrap();
            let group_member = &mut test_group_member.user;

            let group_member_conversation_before =
                group_member.conversation(conversation_id).unwrap();
            assert_eq!(
                group_member_conversation_before.status(),
                &ConversationStatus::Active
            );
            let past_members = HashSet::<UserName>::from_iter(
                group_member.group_members(conversation_id).unwrap(),
            );

            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_member_conversation_after =
                group_member.conversation(conversation_id).unwrap();
            if let ConversationStatus::Inactive(inactive_status) =
                &group_member_conversation_after.status()
            {
                let inactive_status_members =
                    HashSet::<UserName>::from_iter(inactive_status.past_members().to_vec());
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
        }
        self.groups.remove(&conversation_id);

        self.flush_notifications();
    }

    pub fn random_user(&self, rng: &mut impl RngCore) -> UserName {
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
                if let Some(other_user) = self
                    .users
                    .keys()
                    .filter(|&other_user| {
                        let is_contact = self
                            .users
                            .get(other_user)
                            .unwrap()
                            .user()
                            .contacts()
                            .unwrap()
                            .into_iter()
                            .any(|contact| contact.user_name == random_user);
                        // The other user can't be the same user and the other
                        //  user can't already be connected
                        other_user != &random_user && !is_contact
                    })
                    .choose(rng)
                {
                    tracing::info!(
                        random_operation = true,
                        "Random operation: Connecting {} and {}",
                        random_user,
                        other_user
                    );
                    self.connect_users(random_user, other_user.clone()).await;
                }
            }
            1 => {
                let conversation_id = self.create_group(random_user).await;
                tracing::info!(
                    random_operation = true,
                    "Random operation: Created group {}",
<<<<<<< HEAD
                    conversation_id
=======
                    conversation_id.as_uuid()
>>>>>>> main
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
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
<<<<<<< HEAD
                        conversation.conversation_type == ConversationType::Group
                            && conversation.status == ConversationStatus::Active
=======
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
>>>>>>> main
                    })
                    .choose(rng)
                {
                    let number_of_invitees = rng.gen_range(1..=5);
                    let invitee_names = self
                        .users
                        .keys()
                        .filter(|&invitee| {
                            // The invitee user can't be already in the group, can't
                            // be the random user and must be connected
                            let is_group_member = self
                                .groups
<<<<<<< HEAD
                                .get(&conversation.id.as_uuid())
=======
                                .get(&conversation.id())
>>>>>>> main
                                .unwrap()
                                .contains(invitee);
                            let is_connected = user
                                .user()
                                .contacts()
                                .unwrap()
                                .into_iter()
                                .any(|contact| &contact.user_name == invitee);
                            !is_group_member && is_connected && invitee != &random_user
                        })
                        .cloned()
                        .choose_multiple(rng, number_of_invitees);
                    // It can happen that there are no suitable users to invite
                    if invitee_names.len() > 0 {
                        let invitee_strings = invitee_names
                            .iter()
                            .map(|invitee| invitee.to_string())
                            .collect::<Vec<_>>();
                        tracing::info!(
                            random_operation = true,
                            "Random operation: {} invites {} to group {}",
                            random_user,
                            invitee_strings.join(", "),
<<<<<<< HEAD
                            conversation.id.as_uuid()
                        );
                        self.invite_to_group(conversation.id.as_uuid(), random_user, invitee_names)
=======
                            conversation.id().as_uuid()
                        );
                        self.invite_to_group(conversation.id(), random_user, invitee_names)
>>>>>>> main
                            .await;
                    }
                }
            }
            3 => {
                let user = self.users.get(&random_user).unwrap();
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
<<<<<<< HEAD
                        conversation.conversation_type == ConversationType::Group
                            && conversation.status == ConversationStatus::Active
=======
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
>>>>>>> main
                    })
                    .choose(rng)
                {
                    let number_of_removals = rng.gen_range(1..=5);
                    let members_to_remove = self
                        .groups
<<<<<<< HEAD
                        .get(&conversation.id.as_uuid())
=======
                        .get(&conversation.id())
>>>>>>> main
                        .unwrap()
                        .iter()
                        .filter(|&member| member != &random_user)
                        .cloned()
                        .choose_multiple(rng, number_of_removals);
                    if members_to_remove.len() > 0 {
                        let removed_strings = members_to_remove
                            .iter()
                            .map(|removed| removed.to_string())
                            .collect::<Vec<_>>();
                        tracing::info!(
                            random_operation = true,
                            "Random operation: {} removes {} from group {}",
                            random_user,
                            removed_strings.join(", "),
<<<<<<< HEAD
                            conversation.id.as_uuid()
                        );
                        self.remove_from_group(
                            conversation.id.as_uuid(),
                            random_user,
                            members_to_remove,
                        )
                        .await;
=======
                            conversation.id().as_uuid()
                        );
                        self.remove_from_group(conversation.id(), random_user, members_to_remove)
                            .await;
>>>>>>> main
                    }
                }
            }
            4 => {
                let user = self.users.get(&random_user).unwrap();
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
<<<<<<< HEAD
                        conversation.conversation_type == ConversationType::Group
                            && conversation.status == ConversationStatus::Active
=======
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
>>>>>>> main
                    })
                    .choose(rng)
                {
                    tracing::info!(
                        random_operation = true,
                        "Random operation: {} leaves group {}",
                        random_user,
<<<<<<< HEAD
                        conversation.id.as_uuid()
                    );
                    self.leave_group(conversation.id.as_uuid(), random_user)
                        .await;
=======
                        conversation.id().as_uuid()
                    );
                    self.leave_group(conversation.id(), random_user).await;
>>>>>>> main
                }
            }
            _ => panic!("Invalid action"),
        }
    }
}
