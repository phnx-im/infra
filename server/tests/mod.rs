// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;
mod utils;

use phnxapiclient::{ApiClient, TransportEncryption};

use phnxserver::network_provider::MockNetworkProvider;
use utils::setup::TestBackend;
pub use utils::*;

#[actix_rt::test]
#[tracing::instrument(name = "Test WS", skip_all)]
async fn health_check_works() {
    tracing::info!("Tracing: Spawning websocket connection task");
    let network_provider = MockNetworkProvider::new();
    let (address, _ws_dispatch) = spawn_app("example.com".into(), network_provider, true).await;

    tracing::info!("Server started: {}", address.to_string());

    // Initialize the client
    let client = ApiClient::initialize(address, TransportEncryption::Off)
        .expect("Failed to initialize client");

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
    let (address, _ws_dispatch) = spawn_app("localhost".into(), network_provider, true).await;

    // Initialize the client
    let client = ApiClient::initialize(address, TransportEncryption::Off)
        .expect("Failed to initialize client");

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
