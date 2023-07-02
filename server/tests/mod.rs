// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;
mod utils;

use std::sync::{Arc, Mutex};

use phnxapiclient::{ApiClient, TransportEncryption};
use phnxcoreclient::{
    notifications::{self, Notifiable, NotificationHub, Notifier},
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

    let mut alice_notifier = TestNotifier::new();
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

    // Alice adds Bob as a contact
    tracing::info!("Alice adds Bob as a contact");
    alice.add_contact("bob").await;

    assert!(alice.contacts().is_empty());
    assert_eq!(alice.partial_contacts().len(), 1);

    assert_eq!(&alice.partial_contacts()[0].user_name.to_string(), "bob");
    assert_eq!(alice.get_conversations().len(), 1);

    // Bob fetches messages from the AS
    tracing::info!("Bob fetches messages from the AS");
    let as_messages = bob.as_fetch_messages().await;
    bob.process_as_messages(as_messages).await.unwrap();

    tracing::info!("Alice fetches her messages from the QS");
    let qs_messages = alice.qs_fetch_messages().await;
    alice.process_qs_messages(qs_messages).await.unwrap();

    // Check that the contact is no longer pending
    assert_eq!(alice.contacts().len(), 1);
    assert!(alice.partial_contacts().is_empty());

    assert_eq!(&alice.contacts()[0].user_name.to_string(), "bob");
    assert_eq!(alice.get_conversations().len(), 1);

    assert_eq!(
        alice.get_conversations()[0].status,
        ConversationStatus::Active
    );

    assert_eq!(
        bob.get_conversations()[0].status,
        ConversationStatus::Active
    );

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

    return;

    tracing::info!("Alice creates a conversation with Bob");
    let conversation_id = alice
        .create_conversation("Conversation Alice/Bob")
        .await
        .unwrap();

    let alice_conversations = alice.get_conversations();

    println!("Alice's conversations: {:?}", alice_conversations);

    assert_eq!(alice_conversations.len(), 2);
    assert!(matches!(
        alice_conversations[0].conversation_type,
        ConversationType::UnconfirmedConnection(_)
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
    alice.add_contact("charlie").await;

    assert_eq!(alice.contacts().len(), 1);
    assert_eq!(alice.partial_contacts().len(), 1);

    assert_eq!(
        &alice.partial_contacts()[0].user_name.to_string(),
        "charlie"
    );
    assert_eq!(alice.get_conversations().len(), 3);

    // Charlie fetches messages from the AS
    tracing::info!("Charlie fetches messages from the AS");
    let as_messages = charlie.as_fetch_messages().await;
    charlie.process_as_messages(as_messages).await.unwrap();

    // Check charlie has added Alice
    assert_eq!(charlie.contacts().len(), 1);
    assert!(charlie.partial_contacts().is_empty());

    assert_eq!(&charlie.contacts()[1].user_name.to_string(), "alice");
    assert_eq!(charlie.get_conversations().len(), 1);

    tracing::info!("Alice fetches her messages from the QS");
    let qs_messages = alice.qs_fetch_messages().await;
    alice.process_qs_messages(qs_messages).await.unwrap();

    // Check that the contact is no longer pending
    assert_eq!(alice.contacts().len(), 2);
    assert!(alice.partial_contacts().is_empty());

    assert_eq!(&alice.contacts()[1].user_name.to_string(), "charlie");
    assert_eq!(alice.get_conversations().len(), 3);

    assert_eq!(
        alice.get_conversations()[2].status,
        ConversationStatus::Active
    );

    assert_eq!(
        charlie.get_conversations()[0].status,
        ConversationStatus::Active
    );

    // Alice adds charlie to her conversation with bob
    tracing::info!("Alice invites Charlie");
    alice
        .invite_users(&conversation_id, &["charlie"])
        .await
        .unwrap();

    // Charlie fetches messages from the QS to accept the invitation
    tracing::info!("Charlie fetches messages from the AS");
    let qs_messages = charlie.qs_fetch_messages().await;
    charlie.process_qs_messages(qs_messages).await.unwrap();

    assert_eq!(charlie.get_conversations().len(), 2);
}
