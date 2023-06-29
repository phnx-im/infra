// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;
mod utils;

use phnxapiclient::{ApiClient, TransportEncryption};
use phnxcoreclient::{
    notifications::{Notifiable, NotificationHub},
    types::NotificationType,
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
struct TestNotifier;

impl Notifiable for TestNotifier {
    fn notify(&self, _notification_type: NotificationType) -> bool {
        true
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

#[should_panic]
#[actix_rt::test]
#[tracing::instrument(name = "Full cycle", skip_all)]
async fn full_cycle() {
    let (address, _ws_dispatch) = spawn_app().await;

    let notification_hub_alice = NotificationHub::<TestNotifier>::default();
    let notification_hub_bob = NotificationHub::<TestNotifier>::default();

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
    panic!()

    //tracing::info!("Alice creates a conversation with Bob");
    //let conversation_id = alice
    //    .create_conversation("Conversation Alice/Bob")
    //    .await
    //    .unwrap();

    //tracing::info!("Alice invites Bob");
    //alice
    //    .invite_users(&conversation_id, &["bob"])
    //    .await
    //    .unwrap();
}
