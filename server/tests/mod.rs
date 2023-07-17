// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;
mod utils;

use phnxapiclient::{ApiClient, TransportEncryption};

use utils::setup::TestBackend;
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

#[actix_rt::test]
#[tracing::instrument(name = "Connect users test", skip_all)]
async fn connect_users() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.add_user("bob").await;
    setup.connect_users("alice", "bob").await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Send message test", skip_all)]
async fn send_message() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.add_user("bob").await;
    let conversation_id = setup.connect_users("alice", "bob").await;
    setup.send_message(conversation_id, "alice", &["bob"]).await;
    setup.send_message(conversation_id, "bob", &["alice"]).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Create group test", skip_all)]
async fn create_group() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.create_group("alice").await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn invite_to_group() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.add_user("bob").await;
    setup.add_user("charlie").await;
    setup.connect_users("alice", "bob").await;
    setup.connect_users("alice", "charlie").await;
    let conversation_id = setup.create_group("alice").await;
    setup
        .invite_to_group(conversation_id, "alice", &["bob", "charlie"])
        .await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn update_group() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.add_user("bob").await;
    setup.add_user("charlie").await;
    setup.connect_users("alice", "bob").await;
    setup.connect_users("alice", "charlie").await;
    let conversation_id = setup.create_group("alice").await;
    setup
        .invite_to_group(conversation_id, "alice", &["bob", "charlie"])
        .await;
    setup.update_group(conversation_id, "bob").await
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn remove_from_group() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.add_user("bob").await;
    setup.add_user("charlie").await;
    setup.add_user("dave").await;
    setup.connect_users("alice", "bob").await;
    setup.connect_users("alice", "charlie").await;
    setup.connect_users("alice", "dave").await;
    let conversation_id = setup.create_group("alice").await;
    setup
        .invite_to_group(conversation_id, "alice", &["bob", "charlie", "dave"])
        .await;
    setup
        .remove_from_group(conversation_id, "charlie", &["alice", "bob"])
        .await
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn leave_group() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
    setup.add_user("bob").await;
    setup.connect_users("alice", "bob").await;
    let conversation_id = setup.create_group("alice").await;
    setup
        .invite_to_group(conversation_id, "alice", &["bob"])
        .await;
    setup.leave_group(conversation_id, "alice").await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Create user", skip_all)]
async fn create_user() {
    let mut setup = TestBackend::new().await;
    setup.add_user("alice").await;
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
    let mut setup = TestBackend::new().await;
    // Create alice and bob
    setup.add_user("alice").await;
    setup.add_user("bob").await;

    // Connect them
    let conversation_alice_bob = setup.connect_users("alice", "bob").await;

    // Test the connection conversation by sending messages back and forth.
    setup
        .send_message(conversation_alice_bob, "alice", &["bob"])
        .await;
    setup
        .send_message(conversation_alice_bob, "bob", &["alice"])
        .await;

    // Create an independent group and invite bob.
    let conversation_id = setup.create_group("alice").await;

    setup
        .invite_to_group(conversation_id, "alice", &["bob"])
        .await;

    // Create chalie, connect him with alice and invite him to the group.
    setup.add_user("charlie").await;
    setup.connect_users("alice", "charlie").await;

    setup
        .invite_to_group(conversation_id, "alice", &["charlie"])
        .await;

    // Add dave, connect him with charlie and invite him to the group. Then have dave remove alice and bob.
    setup.add_user("dave").await;
    setup.connect_users("charlie", "dave").await;

    setup
        .invite_to_group(conversation_id, "charlie", &["dave"])
        .await;

    setup
        .send_message(conversation_id, "alice", &["charlie", "bob", "dave"])
        .await;

    setup
        .remove_from_group(conversation_id, "dave", &["alice", "bob"])
        .await;

    setup.leave_group(conversation_id, "charlie").await
}

#[actix_rt::test]
async fn benchmarks() {
    let mut setup = TestBackend::new().await;

    const NUM_USERS: usize = 10;
    const NUM_MESSAGES: usize = 10;

    // Create alice
    setup.add_user("alice").await;

    // Create bob
    setup.add_user("bob").await;

    // Create many different bobs
    let bobs: Vec<String> = (0..NUM_USERS)
        .map(|i| format!("bob{}", i))
        .collect::<Vec<String>>();

    // Measure the time it takes to create all the users
    let start = std::time::Instant::now();
    for bob in &bobs {
        setup.add_user(&bob).await;
    }
    let elapsed = start.elapsed();
    println!(
        "Creating {} users took {}ms on average",
        NUM_USERS,
        elapsed.as_millis() / NUM_USERS as u128
    );

    // Measure the time it takes to connect all bobs with alice
    let start = std::time::Instant::now();
    for bob in &bobs {
        setup.connect_users("alice", &bob).await;
    }
    let elapsed = start.elapsed();
    println!(
        "Connecting {} users took {}ms on average",
        NUM_USERS,
        elapsed.as_millis() / NUM_USERS as u128
    );

    // Connect them
    let conversation_alice_bob = setup.connect_users("alice", "bob").await;

    // Measure the time it takes to send a message
    let start = std::time::Instant::now();
    for _ in 0..NUM_MESSAGES {
        setup
            .send_message(conversation_alice_bob, "alice", &["bob"])
            .await;
    }
    let elapsed = start.elapsed();
    println!(
        "Sending {} messages in a connection group took {}ms on average",
        NUM_MESSAGES,
        elapsed.as_millis() / NUM_MESSAGES as u128
    );

    // Create an independent group
    let conversation_id = setup.create_group("alice").await;

    // Measure the time it takes to invite a user
    let start = std::time::Instant::now();
    for bob in &bobs {
        setup
            .invite_to_group(conversation_id, "alice", &[&bob])
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
            .send_message(
                conversation_id,
                "alice",
                bobs.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .as_slice(),
            )
            .await;
    }
    let elapsed = start.elapsed();
    println!(
        "Sending {} messages in an independent group took {}ms on average",
        NUM_MESSAGES,
        elapsed.as_millis() / NUM_MESSAGES as u128
    );
}
