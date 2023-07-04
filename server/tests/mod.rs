// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;
mod utils;

use std::sync::{Arc, Mutex};

use phnxapiclient::{ApiClient, TransportEncryption};
use phnxcoreclient::{
    notifications::{Notifiable, NotificationHub},
    types::{
        ContentMessage, ConversationStatus, ConversationType, Message, MessageContentType,
        NotificationType,
    },
    users::SelfUser,
};

pub use utils::*;

#[actix_rt::test]
#[tracing::instrument(name = "Test WS", skip_all)]
async fn health_check_works() {
    tracing::info!("Tracing: Spawning websocket connection task");
    let (address, _ws_dispatch) = spawn_app().await;

    tracing::info!("Server started: {}", address.to_string());

    // Initialize the client
    let client = ApiClient::initialize(address, TransportEncryption::Off)
        .expect("Failed to initialize client");

    // Do the health check
    assert!(client.health_check().await);
}

#[derive(Clone)]
struct TestNotifier {
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
        let notifications = self.notifications.lock().unwrap().clone();
        self.notifications = Arc::new(Mutex::new(Vec::new()));
        notifications
    }
}

#[actix_rt::test]
#[tracing::instrument(name = "Create user", skip_all)]
async fn create_user() {
    let (address, _ws_dispatch) = spawn_app().await;
    let notification_hub = NotificationHub::<TestNotifier>::default();

    // Create a user
    let _user = SelfUser::new("testuser", "testpassword", address, notification_hub).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Inexistant endpoint", skip_all)]
async fn inexistant_endpoint() {
    let (address, _ws_dispatch) = spawn_app().await;

    // Initialize the client
    let client = ApiClient::initialize(address, TransportEncryption::Off)
        .expect("Failed to initialize client");

    // Call the inexistant endpoint
    assert!(client.inexistant_endpoint().await);
}

#[actix_rt::test]
#[tracing::instrument(name = "Full cycle", skip_all)]
async fn full_cycle() {
    let (address, _ws_dispatch) = spawn_app().await;

    let mut notification_hub_alice = NotificationHub::<TestNotifier>::default();
    let mut notification_hub_bob = NotificationHub::<TestNotifier>::default();

    let alice_notifier = TestNotifier::new();
    notification_hub_alice.add_sink(alice_notifier.notifier());

    let mut bob_notifier = TestNotifier::new();
    notification_hub_bob.add_sink(bob_notifier.notifier());

    // Create a users
    let mut alice = SelfUser::new("alice", "alicepassword", address, notification_hub_alice).await;
    tracing::info!("Created alice");
    let mut bob = SelfUser::new("bob", "bobpassword", address, notification_hub_bob).await;
    tracing::info!("Created bob");

    assert!(alice.get_conversations().is_empty());
    assert!(bob.get_conversations().is_empty());

    connect_users(&mut alice, &mut bob).await;

    // Alice sends a message to Bob
    tracing::info!("Alice sends a message to Bob");
    let orig_message = MessageContentType::Text(phnxcoreclient::types::TextMessage {
        message: b"Hello Bob".to_vec(),
    });
    let message = alice
        .send_message(alice.get_conversations()[0].id, orig_message.clone())
        .await
        .unwrap();

    assert_eq!(
        message.message,
        Message::Content(ContentMessage {
            sender: b"alice".to_vec(),
            content: orig_message.clone()
        })
    );

    tracing::info!("Bob fetches QS messages");
    let bob_qs_messages = bob.qs_fetch_messages().await;

    bob.process_qs_messages(bob_qs_messages).await.unwrap();

    let bob_notifications = bob_notifier.notifications();

    assert_eq!(bob_notifications.len(), 1);

    assert!(matches!(bob_notifications[0], NotificationType::Message(_)));

    if let NotificationType::Message(message) = &bob_notifications[0] {
        assert_eq!(
            message.conversation_message.message,
            Message::Content(ContentMessage {
                sender: b"alice".to_vec(),
                content: orig_message
            })
        );
    }

    tracing::info!("Alice creates a conversation with Bob");
    let conversation_id = alice
        .create_conversation("Conversation Alice/Bob")
        .await
        .unwrap();

    let alice_conversations = alice.get_conversations();

    assert_eq!(alice_conversations.len(), 2);
    assert!(matches!(
        alice_conversations[0].conversation_type,
        ConversationType::Connection(_)
    ));
    assert!(matches!(
        alice_conversations[1].conversation_type,
        ConversationType::Group
    ));

    tracing::info!("Alice invites Bob");
    alice
        .invite_users(&conversation_id, &["bob"])
        .await
        .unwrap();

    tracing::info!("Bob fetches QS messages");
    let bob_qs_messages = bob.qs_fetch_messages().await;
    assert_eq!(bob_qs_messages.len(), 1);

    tracing::info!("Bob processes QS messages");
    bob.process_qs_messages(bob_qs_messages).await.unwrap();

    let notification_hub_charlie = NotificationHub::<TestNotifier>::default();
    let mut charlie = SelfUser::new(
        "charlie",
        "charliepassword",
        address,
        notification_hub_charlie,
    )
    .await;
    tracing::info!("Created Charlie");

    tracing::info!("Alice adds Charlie as a contact");

    connect_users(&mut alice, &mut charlie).await;

    // Alice adds charlie to her conversation with bob
    tracing::info!("Alice invites Charlie");
    alice
        .invite_users(&conversation_id, &["charlie"])
        .await
        .unwrap();

    // Charlie fetches messages from the QS to accept the invitation
    tracing::info!("Charlie fetches messages from the QS");
    let qs_messages = charlie.qs_fetch_messages().await;
    charlie.process_qs_messages(qs_messages).await.unwrap();

    assert_eq!(charlie.get_conversations().len(), 2);

    // Bob fetches messages from the QS to find that charlie has joined the
    // conversation
    tracing::info!("Bob fetches messages from the QS");
    let qs_messages = bob.qs_fetch_messages().await;
    bob.process_qs_messages(qs_messages).await.unwrap();

    assert_eq!(bob.get_conversations().len(), 2);
}

async fn connect_users<T: Notifiable, U: Notifiable>(
    user1: &mut SelfUser<T>,
    user2: &mut SelfUser<U>,
) {
    let user1_partial_contacts_before = user1.partial_contacts();
    let user1_conversations_before = user1.get_conversations();
    user1.add_contact(&user2.user_name().to_string()).await;
    let mut user1_partial_contacts_after = user1.partial_contacts();
    let new_user_position = user1_partial_contacts_after
        .iter()
        .position(|c| &c.user_name == user2.user_name())
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
        .position(|c| &c.attributes.title == &user2.user_name().to_string())
        .expect("User 1 should have created a new conversation");
    let conversation = user1_conversations_after.remove(new_conversation_position);
    assert!(conversation.status == ConversationStatus::Active);
    assert!(
        conversation.conversation_type
            == ConversationType::UnconfirmedConnection(user2.user_name().as_bytes().to_vec())
    );
    user1_conversations_before
        .into_iter()
        .zip(user1_conversations_after)
        .for_each(|(before, after)| {
            assert_eq!(before.id, after.id);
        });

    let user2_contacts_before = user2.contacts();
    let user2_conversations_before = user2.get_conversations();
    let as_messages = user2.as_fetch_messages().await;
    user2.process_as_messages(as_messages).await.unwrap();
    // User 2 should have auto-accepted (for now at least) the connection request.
    let mut user2_contacts_after = user2.contacts();
    let new_contact_position = user2_contacts_after
        .iter()
        .position(|c| &c.user_name == user1.user_name())
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
        .position(|c| &c.attributes.title == &user1.user_name().to_string())
        .expect("User 2 should have created a new conversation");
    let conversation = user2_conversations_after.remove(new_conversation_position);
    assert!(conversation.status == ConversationStatus::Active);
    assert!(
        conversation.conversation_type
            == ConversationType::Connection(user1.user_name().as_bytes().to_vec())
    );
    user2_conversations_before
        .into_iter()
        .zip(user2_conversations_after)
        .for_each(|(before, after)| {
            assert_eq!(before.id, after.id);
        });

    let user1_contacts_before = user1.contacts();
    let user1_conversations_before = user1.get_conversations();
    let qs_messages = user1.qs_fetch_messages().await;
    user1.process_qs_messages(qs_messages).await.unwrap();

    // User 1 should have added user 2 to its contacts now and a connection
    // group should have been created.
    let mut user1_contacts_after = user1.contacts();
    let new_user_position = user1_contacts_after
        .iter()
        .position(|c| &c.user_name == user2.user_name())
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
        .position(|c| &c.attributes.title == &user2.user_name().to_string())
        .expect("User 1 should have created a new conversation");
    let conversation = user1_conversations_after.remove(new_conversation_position);
    assert!(conversation.status == ConversationStatus::Active);
    assert!(
        conversation.conversation_type
            == ConversationType::Connection(user2.user_name().as_bytes().to_vec())
    );
    user1_conversations_before
        .into_iter()
        .zip(user1_conversations_after)
        .for_each(|(before, after)| {
            assert_eq!(before.id, after.id);
        });
}
