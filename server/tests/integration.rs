// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fs, io::Cursor, sync::LazyLock, time::Duration};

use image::{ImageBuffer, Rgba};
use mimi_content::MimiContent;
use phnxapiclient::{as_api::AsRequestError, ds_api::DsRequestError};
use phnxprotos::{
    auth_service::v1::auth_service_server, delivery_service::v1::delivery_service_server,
    queue_service::v1::queue_service_server,
};
use rand::{Rng, distributions::Alphanumeric, rngs::OsRng};

use phnxcoreclient::{
    Asset, ConversationId, ConversationMessage, DisplayName, UserProfile, clients::CoreUser,
    store::Store,
};
use phnxserver::RateLimitsConfig;
use phnxserver_test_harness::utils::setup::{TestBackend, TestUser};
use phnxtypes::identifiers::QualifiedUserName;
use png::Encoder;
use tonic::transport::Channel;
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

static ALICE: LazyLock<QualifiedUserName> = LazyLock::new(|| "alice@example.com".parse().unwrap());
static BOB: LazyLock<QualifiedUserName> = LazyLock::new(|| "bob@example.com".parse().unwrap());
static CHARLIE: LazyLock<QualifiedUserName> =
    LazyLock::new(|| "charlie@example.com".parse().unwrap());
static DAVE: LazyLock<QualifiedUserName> = LazyLock::new(|| "dave@example.com".parse().unwrap());

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Connect users test", skip_all)]
async fn connect_users() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.connect_users(&ALICE, &BOB).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Rate limit test", skip_all)]
async fn rate_limit() {
    init_test_tracing();

    let mut setup = TestBackend::single_with_rate_limits(RateLimitsConfig {
        period: Duration::from_secs(1), // replenish one token every 500ms
        burst_size: 30,                 // allow total 30 request
    })
    .await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    let conversation_id = setup.connect_users(&ALICE, &BOB).await;

    let alice = setup.users.get_mut(&ALICE).unwrap();

    let mut resource_exhausted = false;

    // should stop with `resource_exhausted = true` at some point
    for i in 0..100 {
        info!(i, "sending message");
        let res = alice
            .user
            .send_message(
                conversation_id,
                MimiContent::simple_markdown_message("Hello bob".into()),
            )
            .await;

        let Err(error) = res else {
            continue;
        };

        let error: DsRequestError = error.downcast().expect("should be a DsRequestError");
        match error {
            DsRequestError::Tonic(status) => {
                assert_eq!(status.code(), tonic::Code::ResourceExhausted);
                resource_exhausted = true;
                break;
            }
            _ => panic!("unexpected error type: {error:?}"),
        }
    }
    assert!(resource_exhausted);

    info!("waiting for rate limit tokens to replenish");
    tokio::time::sleep(Duration::from_secs(1)).await; // replenish

    info!("sending message after rate limit tokens replenished");
    let res = alice
        .user
        .send_message(
            conversation_id,
            MimiContent::simple_markdown_message("Hello bob".into()),
        )
        .await;

    if let Err(error) = res {
        panic!("rate limit did not replenish: {error:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Create group test", skip_all)]
async fn create_group() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.create_group(&ALICE).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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
    assert!(charlie_user_profile_bob.user_name == *BOB);

    setup
        .remove_from_group(conversation_id, &ALICE, vec![&BOB])
        .await;

    // Now that charlie is not in a group with Bob anymore, the user profile
    // should be removed.
    let charlie = setup.get_user(&CHARLIE);
    let charlie_user_profile_bob = charlie.user.user_profile(&BOB).await.unwrap();
    assert!(charlie_user_profile_bob.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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
    setup.leave_group(conversation_id, &BOB).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Invite to group test", skip_all)]
async fn delete_group() {
    init_test_tracing();

    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.connect_users(&ALICE, &BOB).await;
    let conversation_id = setup.create_group(&ALICE).await;
    setup
        .invite_to_group(conversation_id, &ALICE, vec![&BOB])
        .await;
    let delete_group = setup.delete_group(conversation_id, &ALICE);
    delete_group.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Create user", skip_all)]
async fn create_user() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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

    // Add dave, connect him with charlie and invite him to the group. Then have alice remove dave and bob.
    setup.add_user(&DAVE).await;
    setup.connect_users(&CHARLIE, &DAVE).await;

    setup
        .invite_to_group(conversation_id, &CHARLIE, vec![&DAVE])
        .await;

    setup
        .send_message(conversation_id, &ALICE, vec![&CHARLIE, &BOB, &DAVE])
        .await;

    setup
        .remove_from_group(conversation_id, &ALICE, vec![&DAVE, &BOB])
        .await;

    setup.leave_group(conversation_id, &CHARLIE).await;

    setup.delete_group(conversation_id, &ALICE).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "User profile exchange test", skip_all)]
async fn exchange_user_profiles() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;

    // Set a user profile for alice
    let alice_display_name: DisplayName = "4l1c3".parse().unwrap();

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

    let alice_profile = UserProfile {
        user_name: (*ALICE).clone(),
        display_name: alice_display_name.clone(),
        profile_picture: Some(alice_profile_picture.clone()),
    };
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
    let bob_display_name: DisplayName = "B0b".parse().unwrap();
    let bob_profile_picture = Asset::Value(png_bytes.clone());
    let bob_user_profile = UserProfile {
        user_name: (*BOB).clone(),
        display_name: bob_display_name.clone(),
        profile_picture: Some(bob_profile_picture.clone()),
    };

    let user = &setup.users.get(&BOB).unwrap().user;
    user.set_own_user_profile(bob_user_profile).await.unwrap();
    let new_profile = user.own_user_profile().await.unwrap();
    let Asset::Value(compressed_profile_picture) = new_profile.profile_picture.unwrap().clone();

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
        .profile_picture
        .unwrap()
        .clone()
        .value()
        .unwrap()
        .to_vec();

    assert_eq!(profile_picture, compressed_profile_picture);

    assert!(bob_user_profile.display_name == bob_display_name);

    let alice = &mut setup.users.get_mut(&ALICE).unwrap().user;

    let alice_user_profile = alice.user_profile(&ALICE).await.unwrap().unwrap();

    assert_eq!(alice_user_profile.display_name, alice_display_name);

    let new_user_profile = UserProfile {
        user_name: (*ALICE).clone(),
        display_name: "New Alice".parse().unwrap(),
        profile_picture: None,
    };

    alice
        .set_own_user_profile(new_user_profile.clone())
        .await
        .unwrap();

    let bob = &mut setup.users.get_mut(&BOB).unwrap().user;
    let qs_messages = bob.qs_fetch_messages().await.unwrap();
    bob.fully_process_qs_messages(qs_messages).await.unwrap();
    let alice_user_profile = bob.user_profile(&ALICE).await.unwrap().unwrap();

    assert_eq!(alice_user_profile, new_user_profile);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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
        .messages(conversation_id, number_of_messages)
        .await
        .unwrap();

    assert_eq!(messages_retrieved.len(), messages_sent.len());
    assert_eq!(messages_retrieved, messages_sent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "User persistence test", skip_all)]
async fn client_persistence() {
    // Create and persist the user.
    let mut setup = TestBackend::single().await;
    setup.add_persisted_user(&ALICE).await;
    let client_id = setup.users.get(&ALICE).unwrap().user.as_client_id();

    let db_path = setup.temp_dir().to_owned();

    // Try to load the user from the database.
    CoreUser::load(client_id.clone(), db_path.to_str().unwrap())
        .await
        .unwrap();

    let client_db_path = db_path.join(format!("{}.db", client_id));
    assert!(client_db_path.exists());

    setup.delete_user(&ALICE).await;

    assert!(!client_db_path.exists());
    assert!(
        CoreUser::load(client_id.clone(), db_path.to_str().unwrap())
            .await
            .is_err()
    );

    // `CoreUser::load` opened the client DB, and so it was re-created.
    fs::remove_file(client_db_path).unwrap();
    fs::remove_file(db_path.join("phnx.db")).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Test server error if unknown user", skip_all)]
async fn error_if_user_doesnt_exist() {
    let mut setup = TestBackend::single().await;

    setup.add_user(&ALICE).await;
    let alice_test = setup.users.get_mut(&ALICE).unwrap();
    let alice = &mut alice_test.user;

    let res = alice.add_contact(BOB.clone()).await;

    assert!(res.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Delete user test", skip_all)]
async fn delete_user() {
    let mut setup = TestBackend::single().await;

    setup.add_user(&ALICE).await;
    // Adding another user with the same name should fail.
    match TestUser::try_new(&ALICE, Some("localhost".into()), setup.grpc_port()).await {
        Ok(_) => panic!("Should not be able to create a user with the same name"),
        Err(e) => match e.downcast_ref::<AsRequestError>().unwrap() {
            AsRequestError::Tonic(status) => {
                assert_eq!(status.code(), tonic::Code::AlreadyExists);
            }
            _ => panic!("Unexpected error type: {e}"),
        },
    }

    setup.delete_user(&ALICE).await;
    // After deletion, adding the user again should work.
    // Note: Since the user is ephemeral, there is nothing to test on the client side.
    TestUser::try_new(&ALICE, Some("localhost".into()), setup.grpc_port())
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[tracing::instrument(name = "Update user profile on group join test", skip_all)]
async fn update_user_profile_on_group_join() {
    let mut setup = TestBackend::single().await;
    setup.add_user(&ALICE).await;
    setup.add_user(&BOB).await;
    setup.add_user(&CHARLIE).await;

    // Alice and Bob are connected.
    let _alice_bob_conversation = setup.connect_users(&ALICE, &BOB).await;
    // Bob and Charlie are connected.
    let _bob_charlie_conversation = setup.connect_users(&BOB, &CHARLIE).await;

    // Alice updates her profile.
    let alice_display_name: DisplayName = "4l1c3".parse().unwrap();
    let alice_profile = UserProfile {
        user_name: (*ALICE).clone(),
        display_name: alice_display_name.clone(),
        profile_picture: None,
    };
    setup
        .users
        .get(&ALICE)
        .unwrap()
        .user
        .set_own_user_profile(alice_profile)
        .await
        .unwrap();

    // Bob doesn't fetch his queue, so he doesn't know about Alice's new profile.
    // He creates a group and invites Charlie.
    let conversation_id = setup.create_group(&BOB).await;

    let bob = setup.users.get_mut(&BOB).unwrap();
    bob.user
        .invite_users(conversation_id, &[CHARLIE.clone()])
        .await
        .unwrap();

    // Charlie accepts the invitation.
    let charlie = setup.users.get_mut(&CHARLIE).unwrap();
    let charlie_qs_messages = charlie.user.qs_fetch_messages().await.unwrap();
    charlie
        .user
        .fully_process_qs_messages(charlie_qs_messages)
        .await
        .unwrap();

    // Bob now invites Alice
    let bob = setup.users.get_mut(&BOB).unwrap();
    bob.user
        .invite_users(conversation_id, &[ALICE.clone()])
        .await
        .unwrap();

    // Charlie processes his messages again, this will fail, because he will
    // unsuccessfully try to download Alice's old profile.
    let charlie = setup.users.get_mut(&CHARLIE).unwrap();
    let charlie_qs_messages = charlie.user.qs_fetch_messages().await.unwrap();
    let result = charlie
        .user
        .fully_process_qs_messages(charlie_qs_messages)
        .await
        .unwrap();

    assert!(result.changed_conversations.is_empty());
    assert!(result.new_conversations.is_empty());
    assert!(result.new_messages.is_empty());
    let err = &result.errors[0];
    let AsRequestError::Tonic(tonic_err) = err.downcast_ref().unwrap() else {
        panic!("Unexpected error type");
    };
    assert_eq!(tonic_err.code(), tonic::Code::InvalidArgument);
    assert_eq!(tonic_err.message(), "No ciphertext matching index");

    // Alice accepts the invitation.
    let alice = setup.users.get_mut(&ALICE).unwrap();
    let alice_qs_messages = alice.user.qs_fetch_messages().await.unwrap();
    alice
        .user
        .fully_process_qs_messages(alice_qs_messages)
        .await
        .unwrap();

    // While processing her messages, Alice should have issued a profile update

    // Charlie picks up his messages.
    let charlie = setup.users.get_mut(&CHARLIE).unwrap();
    let charlie_qs_messages = charlie.user.qs_fetch_messages().await.unwrap();
    charlie
        .user
        .fully_process_qs_messages(charlie_qs_messages)
        .await
        .unwrap();
    // Charlie should now have Alice's new profile.
    let charlie_user_profile = charlie.user.user_profile(&ALICE).await.unwrap().unwrap();
    assert_eq!(charlie_user_profile.display_name, alice_display_name);
}

#[tracing::instrument(name = "Health check test", skip_all)]
async fn health_check() {
    let setup = TestBackend::single().await;
    let endpoint = format!("http://localhost:{}", setup.grpc_port());
    let channel = Channel::from_shared(endpoint)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = HealthClient::new(channel);

    let names = [
        auth_service_server::SERVICE_NAME,
        delivery_service_server::SERVICE_NAME,
        queue_service_server::SERVICE_NAME,
    ];

    for name in names {
        let response = client
            .check(HealthCheckRequest {
                service: name.to_string(),
            })
            .await;
        if let Err(error) = response {
            panic!("Health check failed for service {name}: {error}");
        }
        let response = response.unwrap().into_inner();
        assert_eq!(
            ServingStatus::try_from(response.status).unwrap(),
            ServingStatus::Serving
        );
    }
}

fn init_test_tracing() {
    let _ = tracing_subscriber::fmt::fmt()
        .with_test_writer()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
}
