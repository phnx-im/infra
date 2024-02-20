// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;

use std::fs;

use opaque_ke::rand::{rngs::OsRng, Rng};
use phnxapiclient::ApiClient;

use phnxcoreclient::{users::SelfUser, MessageContentType};
use phnxserver::network_provider::MockNetworkProvider;
use phnxserver_test_harness::utils::{setup::TestBackend, spawn_app};
use phnxtypes::identifiers::{Fqdn, SafeTryInto};

#[actix_rt::test]
#[tracing::instrument(name = "Test WS", skip_all)]
async fn health_check_works() {
    tracing::info!("Tracing: Spawning websocket connection task");
    let network_provider = MockNetworkProvider::new();
    let (address, _ws_dispatch) =
        spawn_app(Fqdn::try_from("example.com").unwrap(), network_provider).await;

    let address = format!("http://{}", address);

    // Initialize the client
    let client = ApiClient::initialize(address).expect("Failed to initialize client");

    // Do the health check
    assert!(client.health_check().await);
}

const ALICE: &str = "alice@example.com";
const BOB: &str = "bob@example.com";
const CHARLIE: &str = "charlie@example.com";
const DAVE: &str = "dave@example.com";

#[actix_rt::test]
#[tracing::instrument(name = "Connect users test", skip_all)]
async fn connect_users() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;
    setup.connect_users(ALICE, BOB).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Send message test", skip_all)]
async fn send_message() {
    tracing::info!("Setting up setup");
    let mut setup = TestBackend::single().await;
    tracing::info!("Creating users");
    setup.add_user(ALICE).await;
    tracing::info!("Created alice");
    setup.add_user(BOB).await;
    let conversation_id = setup.connect_users(ALICE, BOB).await;
    setup.send_message(conversation_id, ALICE, vec![BOB]).await;
    setup.send_message(conversation_id, BOB, vec![ALICE]).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Create group test", skip_all)]
async fn create_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.create_group(ALICE).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn invite_to_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;
    setup.add_user(CHARLIE).await;
    setup.connect_users(ALICE, BOB).await;
    setup.connect_users(ALICE, CHARLIE).await;
    let conversation_id = setup.create_group(ALICE).await;
    setup
        .invite_to_group(conversation_id, ALICE, vec![BOB, CHARLIE])
        .await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn update_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;
    setup.add_user(CHARLIE).await;
    setup.connect_users(ALICE, BOB).await;
    setup.connect_users(ALICE, CHARLIE).await;
    let conversation_id = setup.create_group(ALICE).await;
    setup
        .invite_to_group(conversation_id, ALICE, vec![BOB, CHARLIE])
        .await;
    setup.update_group(conversation_id, BOB).await
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn remove_from_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;
    setup.add_user(CHARLIE).await;
    setup.add_user(DAVE).await;
    setup.connect_users(ALICE, BOB).await;
    setup.connect_users(ALICE, CHARLIE).await;
    setup.connect_users(ALICE, DAVE).await;
    let conversation_id = setup.create_group(ALICE).await;
    setup
        .invite_to_group(conversation_id, ALICE, vec![BOB, CHARLIE, DAVE])
        .await;
    setup
        .remove_from_group(conversation_id, CHARLIE, vec![ALICE, BOB])
        .await
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn leave_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;
    setup.connect_users(ALICE, BOB).await;
    let conversation_id = setup.create_group(ALICE).await;
    setup
        .invite_to_group(conversation_id, ALICE, vec![BOB])
        .await;
    setup.leave_group(conversation_id, ALICE).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn delete_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;
    setup.connect_users(ALICE, BOB).await;
    let conversation_id = setup.create_group(ALICE).await;
    setup
        .invite_to_group(conversation_id, ALICE, vec![BOB])
        .await;
    setup.delete_group(conversation_id, BOB).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Create user", skip_all)]
async fn create_user() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Inexistant endpoint", skip_all)]
async fn inexistant_endpoint() {
    let network_provider = MockNetworkProvider::new();
    let (address, _ws_dispatch) =
        spawn_app(Fqdn::try_from("localhost").unwrap(), network_provider).await;

    // Initialize the client
    let address = format!("http://{}", address);
    let client = ApiClient::initialize(address).expect("Failed to initialize client");

    // Call the inexistant endpoint
    assert!(client.inexistant_endpoint().await);
}

#[actix_rt::test]
#[tracing::instrument(name = "Full cycle", skip_all)]
async fn full_cycle() {
    let mut setup = TestBackend::single().await;
    // Create alice and bob
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;

    // Connect them
    let conversation_alice_bob = setup.connect_users(ALICE, BOB).await;

    // Test the connection conversation by sending messages back and forth.
    setup
        .send_message(conversation_alice_bob, ALICE, vec![BOB])
        .await;
    setup
        .send_message(conversation_alice_bob, BOB, vec![ALICE])
        .await;

    // Create an independent group and invite bob.
    let conversation_id = setup.create_group(ALICE).await;

    setup
        .invite_to_group(conversation_id, ALICE, vec![BOB])
        .await;

    // Create chalie, connect him with alice and invite him to the group.
    setup.add_user(CHARLIE).await;
    setup.connect_users(ALICE, CHARLIE).await;

    setup
        .invite_to_group(conversation_id, ALICE, vec![CHARLIE])
        .await;

    // Add dave, connect him with charlie and invite him to the group. Then have dave remove alice and bob.
    setup.add_user(DAVE).await;
    setup.connect_users(CHARLIE, DAVE).await;

    setup
        .invite_to_group(conversation_id, CHARLIE, vec![DAVE])
        .await;

    setup
        .send_message(conversation_id, ALICE, vec![CHARLIE, BOB, DAVE])
        .await;

    setup
        .remove_from_group(conversation_id, DAVE, vec![ALICE, BOB])
        .await;

    setup.leave_group(conversation_id, CHARLIE).await;

    setup.delete_group(conversation_id, DAVE).await
}

#[actix_rt::test]
async fn benchmarks() {
    let mut setup = TestBackend::single().await;

    const NUM_USERS: usize = 10;
    const NUM_MESSAGES: usize = 10;

    // Create alice
    setup.add_user(ALICE).await;

    // Create bob
    setup.add_user(BOB).await;

    // Create many different bobs
    let bobs: Vec<String> = (0..NUM_USERS)
        .map(|i| format!("bob{}@example.com", i))
        .collect::<Vec<String>>();

    // Measure the time it takes to create all the users
    let start = std::time::Instant::now();
    for bob in bobs.clone() {
        setup.add_user(bob).await;
    }
    let elapsed = start.elapsed();
    println!(
        "Creating {} users took {}ms on average",
        NUM_USERS,
        elapsed.as_millis() / NUM_USERS as u128
    );

    // Measure the time it takes to connect all bobs with alice
    let start = std::time::Instant::now();
    for bob in bobs.clone() {
        setup.connect_users(ALICE, bob).await;
    }
    let elapsed = start.elapsed();
    println!(
        "Connecting {} users took {}ms on average",
        NUM_USERS,
        elapsed.as_millis() / NUM_USERS as u128
    );

    // Connect them
    let conversation_alice_bob = setup.connect_users(ALICE, BOB).await;

    // Measure the time it takes to send a message
    let start = std::time::Instant::now();
    for _ in 0..NUM_MESSAGES {
        setup
            .send_message(conversation_alice_bob, ALICE, vec![BOB])
            .await;
    }
    let elapsed = start.elapsed();
    println!(
        "Sending {} messages in a connection group took {}ms on average",
        NUM_MESSAGES,
        elapsed.as_millis() / NUM_MESSAGES as u128
    );

    // Create an independent group
    let conversation_id = setup.create_group(ALICE).await;

    // Measure the time it takes to invite a user
    let start = std::time::Instant::now();
    for bob in bobs.clone() {
        setup
            .invite_to_group(conversation_id, ALICE, vec![bob])
            .await;
    }
    let elapsed = start.elapsed();
    println!(
        "Inviting {} users took {}ms on average",
        NUM_USERS,
        elapsed.as_millis() / NUM_USERS as u128
    );

    // Measure the time it takes to send a message
    let start = std::time::Instant::now();
    for _ in 0..NUM_MESSAGES {
        setup
            .send_message(conversation_id, ALICE, bobs.clone())
            .await;
    }
    let elapsed = start.elapsed();
    println!(
        "Sending {} messages in an independent group took {}ms on average",
        NUM_MESSAGES,
        elapsed.as_millis() / NUM_MESSAGES as u128
    );
}

#[actix_rt::test]
#[tracing::instrument(name = "User profile exchange test", skip_all)]
async fn exchange_user_profiles() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;

    // Set a user profile for alice
    let alice_display_name = "4l1c3".to_string();
    let alice_profile_picture = vec![0u8, 1, 2, 3, 4, 5];
    setup
        .users
        .get(&SafeTryInto::try_into(ALICE).unwrap())
        .unwrap()
        .user
        .store_user_profile(
            alice_display_name.clone(),
            Some(alice_profile_picture.clone()),
        )
        .await
        .unwrap();

    setup.add_user(BOB).await;

    // Set a user profile for
    let bob_display_name = "B0b".to_string();
    let bob_profile_picture = vec![6u8, 6, 6];
    setup
        .users
        .get(&SafeTryInto::try_into(BOB).unwrap())
        .unwrap()
        .user
        .store_user_profile(bob_display_name.clone(), Some(bob_profile_picture.clone()))
        .await
        .unwrap();

    setup.connect_users(ALICE, BOB).await;

    let bob_contact = setup
        .users
        .get(&SafeTryInto::try_into(ALICE).unwrap())
        .unwrap()
        .user
        .contacts()
        .unwrap()
        .pop()
        .unwrap()
        .clone();

    let profile_picture = bob_contact
        .user_profile()
        .profile_picture_option()
        .unwrap()
        .clone()
        .value()
        .unwrap()
        .to_vec();

    assert!(profile_picture == bob_profile_picture);

    assert!(bob_contact.user_profile().display_name().as_ref() == &bob_display_name);

    let alice_contact = setup
        .users
        .get(&SafeTryInto::try_into(BOB).unwrap())
        .unwrap()
        .user
        .contacts()
        .unwrap()
        .pop()
        .unwrap();

    assert!(alice_contact.user_profile().display_name().as_ref() == &alice_display_name);
}

#[actix_rt::test]
#[tracing::instrument(name = "Message retrieval test", skip_all)]
async fn retrieve_conversation_messages() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;

    let conversation_id = setup.connect_users(ALICE, BOB).await;

    let alice_test_user = setup
        .users
        .get_mut(&SafeTryInto::try_into(ALICE).unwrap())
        .unwrap();
    let alice = &mut alice_test_user.user;

    let number_of_messages = 10;
    let mut messages_sent = vec![];
    for _ in 0..number_of_messages {
        let message: Vec<u8> = OsRng.gen::<[u8; 32]>().to_vec();
        let message_content = MessageContentType::Text(phnxcoreclient::TextMessage::new(message));
        let message = alice
            .send_message(conversation_id, message_content)
            .await
            .unwrap();
        messages_sent.push(message);
    }

    // Reverse the order of the messages, because the messages are retrieved in
    // descending order of timestamps.
    let messages_sent = messages_sent.into_iter().rev().collect::<Vec<_>>();

    // Let's see what Alice's messages for this conversation look like.
    let messages_retrieved = setup
        .users
        .get(&SafeTryInto::try_into(ALICE).unwrap())
        .unwrap()
        .user
        .get_messages(conversation_id, number_of_messages)
        .unwrap();

    assert_eq!(messages_retrieved, messages_sent);
}

#[actix_rt::test]
#[tracing::instrument(name = "Marking messages as read test", skip_all)]
async fn mark_as_read() {
    let mut setup = TestBackend::single().await;
    setup.add_user(ALICE).await;
    setup.add_user(BOB).await;

    let conversation_id = setup.connect_users(ALICE, BOB).await;

    let alice_test_user = setup
        .users
        .get_mut(&SafeTryInto::try_into(ALICE).unwrap())
        .unwrap();
    let alice = &mut alice_test_user.user;

    // Send a few messages
    let number_of_messages = 10;
    let mut messages_sent = vec![];
    for _ in 0..number_of_messages {
        let message: Vec<u8> = OsRng.gen::<[u8; 32]>().to_vec();
        let message_content = MessageContentType::Text(phnxcoreclient::TextMessage::new(message));
        let message = alice
            .send_message(conversation_id, message_content)
            .await
            .unwrap();
        messages_sent.push(message);
    }

    // All messages should be unread
    let expected_unread_message_count = number_of_messages + 2; // 2 because the messages sent by alice and bob to check the connection are also counted.
    let unread_message_count = alice.unread_message_count(conversation_id).unwrap();
    assert_eq!(expected_unread_message_count, unread_message_count);

    // Let's mark all but the last two messages as read (we subtract 2, because
    // the vector is 0-indexed).
    let timestamp = messages_sent[messages_sent.len() - 3].timestamp();

    alice
        .mark_as_read([(&conversation_id, &timestamp)])
        .unwrap();

    // Check if we were successful
    let expected_unread_message_count = 2;
    let unread_message_count = alice.unread_message_count(conversation_id).unwrap();
    assert_eq!(expected_unread_message_count, unread_message_count);
}

#[actix_rt::test]
#[tracing::instrument(name = "User persistence test", skip_all)]
async fn client_persistence() {
    // Create and persist the user.
    let mut setup = TestBackend::single().await;
    setup.add_persisted_user(ALICE).await;
    let client_id = setup
        .users
        .get(&SafeTryInto::try_into(ALICE).unwrap())
        .unwrap()
        .user
        .as_client_id();

    // Try to load the user from the database.
    let user_result = SelfUser::load(client_id.clone(), "./").await.unwrap();

    assert!(user_result.is_some());

    fs::remove_file("./phnx.db").unwrap();
    let client_db_path = format!("./{}.db", client_id);
    fs::remove_file(client_db_path).unwrap();
}
