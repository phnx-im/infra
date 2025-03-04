// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;

use std::{fs, io::Cursor, sync::LazyLock};

use image::{ImageBuffer, Rgba};
use mimi_content::MimiContent;
use opaque_ke::rand::{distributions::Alphanumeric, rngs::OsRng, Rng};
use phnxapiclient::ApiClient;

use phnxcoreclient::{
    clients::CoreUser, store::Store, Asset, ConversationId, ConversationMessage, DisplayName,
    UserProfile,
};
use phnxserver::network_provider::MockNetworkProvider;
use phnxserver_test_harness::utils::{setup::TestBackend, spawn_app};
use phnxtypes::identifiers::QualifiedUserName;
use png::Encoder;

#[actix_rt::test]
#[tracing::instrument(name = "Test WS", skip_all)]
async fn health_check_works() {
    tracing::info!("Tracing: Spawning websocket connection task");
    let network_provider = MockNetworkProvider::new();
    let (address, _ws_dispatch) =
        spawn_app(Some("example.com".parse().unwrap()), network_provider).await;

    let address = format!("http://{}", address);

    // Initialize the client
    let client = ApiClient::with_default_http_client(address).expect("Failed to initialize client");

    // Do the health check
    assert!(client.health_check().await);
}

static ALICE: LazyLock<QualifiedUserName> = LazyLock::new(|| "alice@example.com".parse().unwrap());
static BOB: LazyLock<QualifiedUserName> = LazyLock::new(|| "bob@example.com".parse().unwrap());
static CHARLIE: LazyLock<QualifiedUserName> =
    LazyLock::new(|| "charlie@example.com".parse().unwrap());
static DAVE: LazyLock<QualifiedUserName> = LazyLock::new(|| "dave@example.com".parse().unwrap());

#[actix_rt::test]
#[tracing::instrument(name = "Connect users test", skip_all)]
async fn connect_users() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.connect_users(&ALICE, &BOB).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Send message test", skip_all)]
async fn send_message() {
    tracing::info!("Setting up setup");
    let mut setup = TestBackend::single().await;
    tracing::info!("Creating users");
    setup.add_user(&ALICE).await;
    tracing::info!("Created alice");
    setup.add_user(&BOB).await;
    let conversation_id = setup.connect_users(&ALICE, &BOB).await;
    setup
        .send_message(conversation_id, &ALICE, vec![&BOB])
        .await;
    setup
        .send_message(conversation_id, &BOB, vec![&ALICE])
        .await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Create group test", skip_all)]
async fn create_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.create_group(&ALICE).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn invite_to_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.add_user(&CHARLIE).await;
    setup.connect_users(&ALICE, &BOB).await;
    setup.connect_users(&ALICE, &CHARLIE).await;
    let conversation_id = setup.create_group(&ALICE).await;
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB, &CHARLIE])
        .await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn update_group() {
    let mut setup = TestBackend::single().await;
    tracing::info!("Adding users");
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.add_user(&CHARLIE).await;
    tracing::info!("Connecting users");
    setup.connect_users(&ALICE, &BOB).await;
    setup.connect_users(&ALICE, &CHARLIE).await;
    let conversation_id = setup.create_group(&ALICE).await;
    tracing::info!("Inviting to group");
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB, &CHARLIE])
        .await;
    tracing::info!("Updating group");
    setup.update_group(conversation_id, &BOB).await
}

#[actix_rt::test]
#[tracing::instrument(name = "Remove from group test", skip_all)]
async fn remove_from_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.add_user(&CHARLIE).await;
    setup.add_user(&DAVE).await;
    setup.connect_users(&ALICE, &BOB).await;
    setup.connect_users(&ALICE, &CHARLIE).await;
    setup.connect_users(&ALICE, &DAVE).await;
    let conversation_id = setup.create_group(&ALICE).await;
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB, &CHARLIE, &DAVE])
        .await;
    // Check that Charlie has a user profile stored for BOB, even though
    // he hasn't connected with them.
    let charlie = setup.get_user(&CHARLIE);
    let charlie_user_profile_bob = charlie.user.user_profile(&BOB).await.unwrap().unwrap();
    assert!(charlie_user_profile_bob.user_name() == &*BOB);

    setup
        .remove_from_group(conversation_id, &CHARLIE, vec![&ALICE, &BOB])
        .await;

    // Now that charlie is not in a group with Bob anymore, the user profile
    // should be removed.
    let charlie = setup.get_user(&CHARLIE);
    let charlie_user_profile_bob = charlie.user.user_profile(&BOB).await.unwrap();
    assert!(charlie_user_profile_bob.is_none());
}

#[actix_rt::test]
#[tracing::instrument(name = "Re-add to group test", skip_all)]
async fn re_add_client() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.connect_users(&ALICE, &BOB).await;
    let conversation_id = setup.create_group(&ALICE).await;
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB])
        .await;
    for _ in 0..10 {
        setup
            .remove_from_group(conversation_id, &ALICE, vec![&BOB])
            .await;
        setup
            .invite_to_group(conversation_id, &ALICE, vec![&BOB])
            .await;
    }
    setup
        .send_message(conversation_id, &ALICE, vec![&BOB])
        .await;
    setup
        .send_message(conversation_id, &BOB, vec![&ALICE])
        .await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn leave_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.connect_users(&ALICE, &BOB).await;
    let conversation_id = setup.create_group(&ALICE).await;
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB])
        .await;
    setup.leave_group(conversation_id, &ALICE).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn delete_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.connect_users(&ALICE, &BOB).await;
    let conversation_id = setup.create_group(&ALICE).await;
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB])
        .await;
    let bob = &BOB;
    let delete_group = setup.delete_group(conversation_id, bob);
    delete_group.await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Create user", skip_all)]
async fn create_user() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
}

#[actix_rt::test]
#[tracing::instrument(name = "Inexistant endpoint", skip_all)]
async fn inexistant_endpoint() {
    let network_provider = MockNetworkProvider::new();
    let (address, _ws_dispatch) =
        spawn_app(Some("localhost".parse().unwrap()), network_provider).await;

    // Initialize the client
    let address = format!("http://{}", address);
    let client = ApiClient::with_default_http_client(address).expect("Failed to initialize client");

    // Call the inexistant endpoint
    assert!(client.inexistant_endpoint().await);
}

#[actix_rt::test]
#[tracing::instrument(name = "Full cycle", skip_all)]
async fn full_cycle() {
    let mut setup = TestBackend::single().await;
    // Create alice and bob
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;

    // Connect them
    let conversation_alice_bob = setup.connect_users(&ALICE, &BOB).await;

    // Test the connection conversation by sending messages back and forth.
    setup
        .send_message(conversation_alice_bob, &ALICE, vec![&BOB])
        .await;
    setup
        .send_message(conversation_alice_bob, &BOB, vec![&ALICE])
        .await;

    // Create an independent group and invite bob.
    let conversation_id = setup.create_group(&ALICE).await;

    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB])
        .await;

    // Create chalie, connect him with alice and invite him to the group.
    setup.add_user(&CHARLIE).await;
    setup.connect_users(&ALICE, &CHARLIE).await;

    setup
        .invite_to_group(conversation_id, &ALICE, vec![&CHARLIE])
        .await;

    // Add dave, connect him with charlie and invite him to the group. Then have dave remove alice and bob.
    setup.add_user(&DAVE).await;
    setup.connect_users(&CHARLIE, &DAVE).await;

    setup
        .invite_to_group(conversation_id, &CHARLIE, vec![&DAVE])
        .await;

    setup
        .send_message(conversation_id, &ALICE, vec![&CHARLIE, &BOB, &DAVE])
        .await;

    setup
        .remove_from_group(conversation_id, &DAVE, vec![&ALICE, &BOB])
        .await;

    setup.leave_group(conversation_id, &CHARLIE).await;

    setup.delete_group(conversation_id, &DAVE).await
}

#[actix_rt::test]
async fn benchmarks() {
    let mut setup = TestBackend::single().await;

    const NUM_USERS: usize = 10;
    const NUM_MESSAGES: usize = 10;

    // Create alice
    setup.add_user(&ALICE).await;

    // Create bob
    setup.add_user(&BOB).await;

    // Create many different bobs
    let bobs: Vec<QualifiedUserName> = (0..NUM_USERS)
        .map(|i| format!("bob{i}@example.com").parse().unwrap())
        .collect();

    // Measure the time it takes to create all the users
    let start = std::time::Instant::now();
    for bob in bobs.clone() {
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
    for bob in bobs.clone() {
        setup.connect_users(&ALICE, &bob).await;
    }
    let elapsed = start.elapsed();
    println!(
        "Connecting {} users took {}ms on average",
        NUM_USERS,
        elapsed.as_millis() / NUM_USERS as u128
    );

    // Connect them
    let conversation_alice_bob = setup.connect_users(&ALICE, &BOB).await;

    // Measure the time it takes to send a message
    let start = std::time::Instant::now();
    for _ in 0..NUM_MESSAGES {
        setup
            .send_message(conversation_alice_bob, &ALICE, vec![&BOB])
            .await;
    }
    let elapsed = start.elapsed();
    println!(
        "Sending {} messages in a connection group took {}ms on average",
        NUM_MESSAGES,
        elapsed.as_millis() / NUM_MESSAGES as u128
    );

    // Create an independent group
    let conversation_id = setup.create_group(&ALICE).await;

    // Measure the time it takes to invite a user
    let start = std::time::Instant::now();
    for bob in bobs.clone() {
        setup
            .invite_to_group(conversation_id, &ALICE, vec![&bob])
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
            .send_message(conversation_id, &ALICE, bobs.iter().collect())
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
    setup.add_user(&ALICE).await;

    // Set a user profile for alice
    let alice_display_name = DisplayName::try_from("4l1c3".to_string()).unwrap();

    // Create a new ImgBuf with width: 1px and height: 1px
    let mut img = ImageBuffer::new(200, 200);

    // Put a single pixel in the image
    img.put_pixel(0, 0, Rgba([0u8, 0u8, 255u8, 255u8])); // Blue pixel

    // A Cursor for in-memory writing of bytes
    let mut buffer = Cursor::new(Vec::new());

    {
        // Create a new PNG encoder
        let mut encoder = Encoder::new(&mut buffer, 200, 200);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        // Encode the image data.
        writer.write_image_data(&img).unwrap();
    }

    // Get the PNG data bytes
    let png_bytes = buffer.into_inner();

    let alice_profile_picture = Asset::Value(png_bytes.clone());

    let alice_profile = UserProfile::new(
        (*ALICE).clone(),
        Some(alice_display_name.clone()),
        Some(alice_profile_picture.clone()),
    );
    setup
        .users
        .get(&ALICE)
        .unwrap()
        .user
        .set_own_user_profile(alice_profile)
        .await
        .unwrap();

    setup.add_user(&BOB).await;

    // Set a user profile for
    let bob_display_name = DisplayName::try_from("B0b".to_string()).unwrap();
    let bob_profile_picture = Asset::Value(png_bytes.clone());
    let bob_user_profile = UserProfile::new(
        (*BOB).clone(),
        Some(bob_display_name.clone()),
        Some(bob_profile_picture.clone()),
    );

    let user = &setup.users.get(&BOB).unwrap().user;
    user.set_own_user_profile(bob_user_profile).await.unwrap();
    let new_profile = user.own_user_profile().await.unwrap();
    let Asset::Value(compressed_profile_picture) = new_profile.profile_picture().unwrap().clone();

    setup.connect_users(&ALICE, &BOB).await;

    let bob_user_profile = setup
        .users
        .get(&ALICE)
        .unwrap()
        .user
        .user_profile(&BOB)
        .await
        .unwrap()
        .unwrap();

    let profile_picture = bob_user_profile
        .profile_picture()
        .unwrap()
        .clone()
        .value()
        .unwrap()
        .to_vec();

    assert_eq!(profile_picture, compressed_profile_picture);

    assert!(bob_user_profile.display_name().unwrap() == &bob_display_name);

    let alice_user_profile = setup
        .users
        .get(&BOB)
        .unwrap()
        .user
        .user_profile(&ALICE)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        alice_user_profile.display_name().unwrap(),
        &alice_display_name
    );
}

#[actix_rt::test]
#[tracing::instrument(name = "Message retrieval test", skip_all)]
async fn retrieve_conversation_messages() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;

    let conversation_id = setup.connect_users(&ALICE, &BOB).await;

    let alice_test_user = setup.users.get_mut(&ALICE).unwrap();
    let alice = &mut alice_test_user.user;

    let number_of_messages = 10;
    let mut messages_sent = vec![];
    for _ in 0..number_of_messages {
        let message: String = OsRng
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let message_content = MimiContent::simple_markdown_message(message);
        let message = alice
            .send_message(conversation_id, message_content)
            .await
            .unwrap();
        messages_sent.push(message);
    }

    // Let's see what Alice's messages for this conversation look like.
    let messages_retrieved = setup
        .users
        .get(&ALICE)
        .unwrap()
        .user
        .get_messages(conversation_id, number_of_messages)
        .await
        .unwrap();

    assert_eq!(messages_retrieved.len(), messages_sent.len());
    assert_eq!(messages_retrieved, messages_sent);
}

#[actix_rt::test]
#[tracing::instrument(name = "Marking messages as read test", skip_all)]
async fn mark_as_read() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.add_user(&CHARLIE).await;

    let alice_bob_conversation = setup.connect_users(&ALICE, &BOB).await;
    let bob_charlie_conversation = setup.connect_users(&BOB, &CHARLIE).await;

    let charlie_test_user = setup.users.get_mut(&ALICE).unwrap();
    let alice = &mut charlie_test_user.user;

    // Send a few messages
    async fn send_messages(
        user: &mut CoreUser,
        conversation_id: ConversationId,
        number_of_messages: usize,
    ) -> Vec<ConversationMessage> {
        let mut messages_sent = vec![];
        for _ in 0..number_of_messages {
            let message: String = OsRng
                .sample_iter(&Alphanumeric)
                .take(32)
                .map(char::from)
                .collect();
            let message_content = MimiContent::simple_markdown_message(message);
            let message = user
                .send_message(conversation_id, message_content)
                .await
                .unwrap();
            messages_sent.push(message);
        }
        messages_sent
    }

    let number_of_messages = 10;
    send_messages(alice, alice_bob_conversation, number_of_messages).await;

    let bob_test_user = setup.users.get_mut(&BOB).unwrap();
    let bob = &mut bob_test_user.user;

    // All messages should be unread
    let qs_messages = bob.qs_fetch_messages().await.unwrap();
    bob.fully_process_qs_messages(qs_messages).await.unwrap();
    let expected_unread_message_count = number_of_messages;
    let unread_message_count = bob.unread_messages_count(alice_bob_conversation).await;
    assert_eq!(expected_unread_message_count, unread_message_count);
    let global_unread_message_count = bob.global_unread_messages_count().await.unwrap();
    let expected_global_unread_message_count = expected_unread_message_count;
    assert_eq!(
        expected_global_unread_message_count,
        global_unread_message_count
    );

    // Let's send some messages between bob and charlie s.t. we can test the
    // global unread messages count.
    let charlie_test_user = setup.users.get_mut(&CHARLIE).unwrap();
    let charlie = &mut charlie_test_user.user;
    let messages_sent = send_messages(charlie, bob_charlie_conversation, number_of_messages).await;

    let bob_test_user = setup.users.get_mut(&BOB).unwrap();
    let bob = &mut bob_test_user.user;

    let qs_messages = bob.qs_fetch_messages().await.unwrap();
    let bob_messages_sent = bob.fully_process_qs_messages(qs_messages).await.unwrap();

    // Let's mark all but the last two messages as read (we subtract 3, because
    // the vector is 0-indexed).
    let timestamp = bob_messages_sent.new_messages[messages_sent.len() - 3].timestamp();

    bob.mark_as_read([(bob_charlie_conversation, timestamp)])
        .await
        .unwrap();

    // Check if we were successful
    let expected_unread_message_count = 2;
    let unread_message_count = bob.unread_messages_count(bob_charlie_conversation).await;
    assert_eq!(expected_unread_message_count, unread_message_count);

    // We expect the global unread messages count to be that of both
    // conversations, i.e. the `expected_unread_message_count` plus
    // `number_of_messages`, because none of the messages between alice and
    // charlie had been read.
    let expected_global_unread_message_count = expected_unread_message_count + number_of_messages;
    let global_unread_messages_count = bob.global_unread_messages_count().await.unwrap();
    assert_eq!(
        global_unread_messages_count,
        expected_global_unread_message_count
    );
}

#[actix_rt::test]
#[tracing::instrument(name = "User persistence test", skip_all)]
async fn client_persistence() {
    // Create and persist the user.
    let mut setup = TestBackend::single().await;
    setup.add_persisted_user(&ALICE).await;
    let client_id = setup.users.get(&ALICE).unwrap().user.as_client_id();

    // Try to load the user from the database.
    let user_result = CoreUser::load(client_id.clone(), "./").await.unwrap();

    assert!(user_result.is_some());

    fs::remove_file("./phnx.db").unwrap();
    let client_db_path = format!("./{}.db", client_id);
    fs::remove_file(client_db_path).unwrap();
}

#[actix_rt::test]
#[tracing::instrument(name = "Test server error if unknown user", skip_all)]
async fn error_if_user_doesnt_exist() {
    let mut setup = TestBackend::single().await;

    setup.add_user(&ALICE).await;
    let alice_test = setup.users.get_mut(&ALICE).unwrap();
    let alice = &mut alice_test.user;

    let res = alice.add_contact((*BOB).clone()).await;

    assert!(res.is_err());
}
