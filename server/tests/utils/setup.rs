// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use opaque_ke::rand::{rngs::OsRng, Rng};
use phnxcoreclient::{
    notifications::{Notifiable, NotificationHub},
    types::{
        ContentMessage, ConversationStatus, ConversationType, InactiveConversation, Message,
        MessageContentType, NotificationType,
    },
    users::SelfUser,
};
use uuid::Uuid;

use crate::spawn_app;

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
    user: SelfUser<TestNotifier>,
    notifier: TestNotifier,
}

impl TestUser {
    pub async fn new(user_name: &str, address: SocketAddr) -> Self {
        let mut notification_hub = NotificationHub::<TestNotifier>::default();

        let notifier = TestNotifier::new();
        notification_hub.add_sink(notifier.notifier());
        let user = SelfUser::new(user_name, user_name, address, notification_hub).await;
        Self { user, notifier }
    }

    pub fn user(&self) -> &SelfUser<TestNotifier> {
        &self.user
    }
}

pub struct TestBackend {
    pub users: HashMap<String, TestUser>,
    pub address: SocketAddr,
}

impl TestBackend {
    pub async fn new() -> Self {
        let (address, _ws_dispatch) = spawn_app().await;
        Self {
            users: HashMap::new(),
            address,
        }
    }

    pub async fn add_user(&mut self, user_name: &str) {
        tracing::info!("Creating {user_name}");
        let user = TestUser::new(user_name, self.address).await;
        self.users.insert(user_name.to_owned(), user);
    }

    pub fn flush_notifications(&mut self) {
        self.users.values_mut().for_each(|u| {
            let _ = u.notifier.notifications();
        });
    }

    pub async fn connect_users(&mut self, user1_name: &str, user2_name: &str) -> Uuid {
        tracing::info!("Connecting users {} and {}", user1_name, user2_name);
        let test_user1 = self.users.get_mut(user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_partial_contacts_before = user1.partial_contacts();
        let user1_conversations_before = user1.get_conversations();
        tracing::info!("{} adds {} as a contact", user1_name, user2_name);
        user1.add_contact(&user2_name).await;
        let mut user1_partial_contacts_after = user1.partial_contacts();
        let new_user_position = user1_partial_contacts_after
            .iter()
            .position(|c| &c.user_name.to_string() == user2_name)
            .expect("User 2 should be in the partial contacts list of user 1");
        // If we remove the new user, the partial contact lists should be the same.
        user1_partial_contacts_after.remove(new_user_position);
        user1_partial_contacts_before
            .into_iter()
            .zip(user1_partial_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.user_name, after.user_name);
            });
        let mut user1_conversations_after = user1.get_conversations();
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| &c.attributes.title == user2_name)
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status == ConversationStatus::Active);
        assert!(
            conversation.conversation_type
                == ConversationType::UnconfirmedConnection(user2_name.as_bytes().to_vec())
        );
        user1_conversations_before
            .into_iter()
            .zip(user1_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id, after.id);
            });
        let user1_conversation_id = conversation.id.clone();

        let test_user2 = self.users.get_mut(user2_name).unwrap();
        let user2 = &mut test_user2.user;
        let user2_contacts_before = user2.contacts();
        let user2_conversations_before = user2.get_conversations();
        tracing::info!("{} fetches AS messages", user2_name);
        let as_messages = user2.as_fetch_messages().await;
        tracing::info!("{} processes AS messages", user2_name);
        user2.process_as_messages(as_messages).await.unwrap();
        // User 2 should have auto-accepted (for now at least) the connection request.
        let mut user2_contacts_after = user2.contacts();
        let new_contact_position = user2_contacts_after
            .iter()
            .position(|c| &c.user_name.to_string() == user1_name)
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
        let mut user2_conversations_after = user2.get_conversations();
        let new_conversation_position = user2_conversations_after
            .iter()
            .position(|c| &c.attributes.title == user1_name)
            .expect("User 2 should have created a new conversation");
        let conversation = user2_conversations_after.remove(new_conversation_position);
        assert!(conversation.status == ConversationStatus::Active);
        assert!(
            conversation.conversation_type
                == ConversationType::Connection(user1_name.as_bytes().to_vec())
        );
        user2_conversations_before
            .into_iter()
            .zip(user2_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id, after.id);
            });
        let user2_conversation_id = conversation.id.clone();

        let test_user1 = self.users.get_mut(user1_name).unwrap();
        let user1 = &mut test_user1.user;
        let user1_contacts_before = user1.contacts();
        let user1_conversations_before = user1.get_conversations();
        tracing::info!("{} fetches QS messages", user1_name);
        let qs_messages = user1.qs_fetch_messages().await;
        tracing::info!("{} processes QS messages", user1_name);
        user1.process_qs_messages(qs_messages).await.unwrap();

        // User 1 should have added user 2 to its contacts now and a connection
        // group should have been created.
        let mut user1_contacts_after = user1.contacts();
        let new_user_position = user1_contacts_after
            .iter()
            .position(|c| &c.user_name.to_string() == user2_name)
            .expect("User 2 should be in the contact list of user 1");
        // If we remove the new user, the partial contact lists should be the same.
        user1_contacts_after.remove(new_user_position);
        user1_contacts_before
            .into_iter()
            .zip(user1_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.user_name, after.user_name);
            });
        // User 2 should have created a connection group.
        let mut user1_conversations_after = user1.get_conversations();
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| &c.attributes.title == &user2_name)
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status == ConversationStatus::Active);
        assert!(
            conversation.conversation_type
                == ConversationType::Connection(user2_name.as_bytes().to_vec())
        );
        user1_conversations_before
            .into_iter()
            .zip(user1_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id, after.id);
            });
        self.flush_notifications();
        debug_assert_eq!(user1_conversation_id, user2_conversation_id);

        // Send messages both ways to ensure it works.
        self.send_message(user1_conversation_id, user1_name, &[user2_name])
            .await;
        self.send_message(user1_conversation_id, user2_name, &[user1_name])
            .await;

        user1_conversation_id
    }

    /// Sends a message from the given sender to the given recipients. Before
    /// sending a message, the sender picks up its QS messages to make sure it's
    /// up to date.
    pub async fn send_message(
        &mut self,
        conversation_id: Uuid,
        sender_name: &str,
        recipient_names: &[&str],
    ) {
        tracing::info!(
            "{} sends a message to {}",
            sender_name,
            recipient_names.join(", ")
        );
        let message: Vec<u8> = OsRng.gen::<[u8; 32]>().to_vec();
        let orig_message = MessageContentType::Text(phnxcoreclient::types::TextMessage { message });
        let test_sender = self.users.get_mut(sender_name).unwrap();
        let sender = &mut test_sender.user;

        // Before sending a message, the sender must first fetch and process its QS messages.

        let sender_qs_messages = sender.qs_fetch_messages().await;

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
                sender: test_sender.user.user_name().as_bytes().to_vec(),
                content: orig_message.clone()
            })
        );

        for recipient_name in recipient_names {
            let recipient = self.users.get_mut(recipient_name.to_owned()).unwrap();
            let recipient_user = &mut recipient.user;
            // Flush notifications
            //let _recipient_notifications = recipient.notifier.notifications();
            let recipient_qs_messages = recipient_user.qs_fetch_messages().await;

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
                        sender: sender_user_name.as_bytes().to_vec(),
                        content: orig_message.clone()
                    })
                );
            }
        }
        self.flush_notifications();
    }

    pub async fn create_group(&mut self, user_name: &str) -> Uuid {
        let test_user = self.users.get_mut(user_name).unwrap();
        let user = &mut test_user.user;
        let user_conversations_before = user.get_conversations();

        let group_name = format!("{:?}", OsRng.gen::<[u8; 32]>());
        let conversation_id = user.create_conversation(&group_name).await.unwrap();
        let mut user_conversations_after = user.get_conversations();
        let new_conversation_position = user_conversations_after
            .iter()
            .position(|c| c.attributes.title == group_name)
            .expect("User 1 should have created a new conversation");
        let conversation = user_conversations_after.remove(new_conversation_position);
        assert!(conversation.id == conversation_id);
        assert!(conversation.status == ConversationStatus::Active);
        assert!(conversation.conversation_type == ConversationType::Group);
        user_conversations_before
            .into_iter()
            .zip(user_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id, after.id);
            });
        self.flush_notifications();
        conversation_id
    }

    /// Has the inviter invite the invitees to the given group and has everyone
    /// send and process their messages.
    pub async fn invite_to_group(
        &mut self,
        conversation_id: Uuid,
        inviter_name: &str,
        invitee_names: &[&str],
    ) {
        let test_inviter = self.users.get_mut(inviter_name).unwrap();
        let inviter = &mut test_inviter.user;

        // Before inviting anyone to a group, the inviter must first fetch and
        // process its QS messages.
        let qs_messages = inviter.qs_fetch_messages().await;

        inviter
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        tracing::info!(
            "{} invites {} to the group with id {}",
            inviter_name,
            invitee_names.join(", "),
            conversation_id
        );

        // Perform the invite operation and check that the invitees are now in the group.
        let inviter_group_members_before: HashSet<String> = inviter
            .group_members(&conversation_id)
            .expect("Error getting group members.")
            .into_iter()
            .collect();

        inviter
            .invite_users(&conversation_id, invitee_names)
            .await
            .expect("Error inviting users.");

        let inviter_group_members_after: HashSet<String> = inviter
            .group_members(&conversation_id)
            .expect("Error getting group members.")
            .into_iter()
            .collect();
        let new_members = inviter_group_members_after
            .difference(&inviter_group_members_before)
            .map(|name| name.to_owned())
            .collect::<HashSet<_>>();
        let invitee_set = invitee_names
            .iter()
            .map(|&name| name.to_owned())
            .collect::<HashSet<_>>();
        assert_eq!(new_members, invitee_set);

        for &invitee_name in invitee_names {
            let test_invitee = self.users.get_mut(invitee_name).unwrap();
            let invitee = &mut test_invitee.user;
            let invitee_conversations_before = invitee.get_conversations();
            tracing::info!("Invitee {} is fetching messages.", invitee_name);

            let qs_messages = invitee.qs_fetch_messages().await;

            tracing::info!("Invitee {} is processing messages.", invitee_name);

            invitee
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let mut invitee_conversations_after = invitee.get_conversations();
            let new_conversation_position = invitee_conversations_after
                .iter()
                .position(|c| c.id == conversation_id)
                .expect(&format!("{invitee_name} should have created a new conversation titles {conversation_id}"));
            let conversation = invitee_conversations_after.remove(new_conversation_position);
            assert!(conversation.id == conversation_id);
            assert!(conversation.status == ConversationStatus::Active);
            assert!(conversation.conversation_type == ConversationType::Group);
            invitee_conversations_before
                .into_iter()
                .zip(invitee_conversations_after)
                .for_each(|(before, after)| {
                    assert_eq!(before.id, after.id);
                });
        }
        self.flush_notifications();
        // Now send messages to check that the group works properly. This also
        // ensures that everyone involved has picked up their messages from the
        // QS and that notifications are flushed.
        self.send_message(conversation_id, inviter_name, invitee_names)
            .await;
        for &invitee_name in invitee_names {
            let recipients: Vec<_> = invitee_names
                .iter()
                .filter(|&&name| name != invitee_name)
                .chain([&inviter_name].into_iter())
                .map(|name| name.to_owned())
                .collect();
            self.send_message(conversation_id, invitee_name, recipients.as_slice())
                .await;
        }
    }

    /// Has the inviter invite the invitees to the given group and has everyone
    /// send and process their messages.
    pub async fn remove_from_group(
        &mut self,
        conversation_id: Uuid,
        remover_name: &str,
        removed_names: &[&str],
    ) {
        let test_remover = self.users.get_mut(remover_name).unwrap();
        let remover = &mut test_remover.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = remover.qs_fetch_messages().await;

        remover
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        tracing::info!(
            "{} removes {} from the group with id {}",
            remover_name,
            removed_names.join(", "),
            conversation_id
        );

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let remover_group_members_before: HashSet<String> = remover
            .group_members(&conversation_id)
            .expect("Error getting group members.")
            .into_iter()
            .collect();

        remover
            .remove_users(&conversation_id, removed_names)
            .await
            .expect("Error inviting users.");

        let remover_group_members_after: HashSet<String> = remover
            .group_members(&conversation_id)
            .expect("Error getting group members.")
            .into_iter()
            .collect();
        let removed_members = remover_group_members_before
            .difference(&remover_group_members_after)
            .map(|name| name.to_owned())
            .collect::<HashSet<_>>();
        let removed_set = removed_names
            .iter()
            .map(|&name| name.to_owned())
            .collect::<HashSet<_>>();
        assert_eq!(removed_members, removed_set);

        for &removed_name in removed_names {
            let test_removed = self.users.get_mut(removed_name).unwrap();
            let removed = &mut test_removed.user;
            let removed_conversations_before = removed.get_conversations();
            let past_members: HashSet<_> = removed
                .group_members(&conversation_id)
                .unwrap()
                .into_iter()
                .collect();

            let qs_messages = removed.qs_fetch_messages().await;

            removed
                .process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let removed_conversations_after = removed.get_conversations();
            let conversation = removed_conversations_after
                .iter()
                .find(|c| c.id == conversation_id)
                .expect(&format!(
                    "{removed_name} should have the conversation with id {conversation_id}"
                ));
            assert!(conversation.id == conversation_id);
            if let ConversationStatus::Inactive(inactive_status) = &conversation.status {
                let inactive_status_members: HashSet<_> =
                    inactive_status.past_members.clone().into_iter().collect();
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
            assert!(conversation.conversation_type == ConversationType::Group);
            let error = removed_conversations_before
                .iter()
                .zip(removed_conversations_after.iter())
                .any(|(before, after)| before.id != after.id);
            if error {
                tracing::info!(
                    "Removed user {} has a different set of conversations before and after the remove operation.",
                    removed_name
                );
                tracing::info!("Before: {:?}", removed_conversations_before);
                tracing::info!("After: {:?}", removed_conversations_after);
            }
            assert!(!error)
        }
        self.flush_notifications();
    }

    /// Has the leaver leave to the given group.
    pub async fn leave_group(&mut self, conversation_id: Uuid, leaver_name: &str) {
        let test_leaver = self.users.get_mut(leaver_name).unwrap();
        let leaver = &mut test_leaver.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = leaver.qs_fetch_messages().await;

        leaver
            .process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        tracing::info!(
            "{} leaves the group with id {}",
            leaver_name,
            conversation_id
        );

        let mut leaver_conversations_before = leaver.get_conversations();
        let leaver_conversation_position = leaver_conversations_before
            .iter()
            .position(|c| c.id == conversation_id)
            .expect(&format!(
                "{leaver_name} should have the conversation with id {conversation_id}"
            ));
        let conversation_before = leaver_conversations_before.remove(leaver_conversation_position);

        // Perform the leave operation.
        leaver.leave_group(&conversation_id).await;

        // TODO: There's not much we can check just yet. In the future, I want
        // to track the groups with the TestBackend and have another group
        // member process the leave. Then we can check the group states of all
        // other members. That would also allow us to do more checking in other
        // tests.

        let mut leaver_conversations_after = leaver.get_conversations();
        let leaver_conversation_position = leaver_conversations_after
            .iter()
            .position(|c| c.id == conversation_id)
            .expect(&format!(
                "{leaver_name} should have the conversation with id {conversation_id}"
            ));
        let conversation_after = leaver_conversations_after.remove(leaver_conversation_position);
        assert!(conversation_before.id == conversation_after.id);
        assert!(conversation_before.conversation_type == ConversationType::Group);
        assert!(conversation_after.conversation_type == ConversationType::Group);
        leaver_conversations_before
            .into_iter()
            .zip(leaver_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id, after.id);
            });

        self.flush_notifications();
    }
}
