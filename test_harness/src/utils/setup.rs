// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use aircommon::{
    DEFAULT_PORT_HTTP, OpenMlsRand, RustCrypto,
    identifiers::{Fqdn, UserHandle, UserId},
};
use aircoreclient::{
    ConversationId, ConversationStatus, ConversationType, clients::CoreUser, store::Store, *,
};
use airserver::{RateLimitsConfig, network_provider::MockNetworkProvider};
use anyhow::Context;
use mimi_content::{
    MimiContent,
    content_container::{EncryptionAlgorithm, HashAlgorithm, NestedPartContent},
};
use rand::{Rng, RngCore, distributions::Alphanumeric, seq::IteratorRandom};
use rand_chacha::rand_core::OsRng;
use tempfile::TempDir;
use tokio::{
    task::{LocalEnterGuard, LocalSet},
    time::timeout,
};
use tokio_stream::StreamExt;
use tracing::info;

use crate::utils::spawn_app_with_rate_limits;

use super::TEST_RATE_LIMITS;

#[derive(Debug)]
pub struct TestUser {
    pub user: CoreUser,
    /// If this is an ephemeral user, this is None.
    pub db_dir: Option<String>,
    /// The user handle record of the user if a handle was added.
    pub user_handle_record: Option<UserHandleRecord>,
}

impl AsRef<CoreUser> for TestUser {
    fn as_ref(&self) -> &CoreUser {
        &self.user
    }
}

impl AsMut<CoreUser> for TestUser {
    fn as_mut(&mut self) -> &mut CoreUser {
        &mut self.user
    }
}

impl TestUser {
    pub async fn new(user_id: &UserId, address_option: Option<String>, grpc_port: u16) -> Self {
        Self::try_new(user_id, address_option, grpc_port)
            .await
            .unwrap()
    }

    pub async fn try_new(
        user_id: &UserId,
        address_option: Option<String>,
        grpc_port: u16,
    ) -> anyhow::Result<Self> {
        let hostname_str =
            address_option.unwrap_or_else(|| format!("{}:{}", user_id.domain(), DEFAULT_PORT_HTTP));

        let server_url = format!("http://{hostname_str}").parse().unwrap();

        let user = CoreUser::new_ephemeral(user_id.clone(), server_url, grpc_port, None).await?;

        Ok(Self {
            user,
            db_dir: None,
            user_handle_record: None,
        })
    }

    pub async fn new_persisted(
        user_id: &UserId,
        address_option: Option<String>,
        grpc_port: u16,
        db_dir: &str,
    ) -> Self {
        let hostname_str =
            address_option.unwrap_or_else(|| format!("{}:{}", user_id.domain(), DEFAULT_PORT_HTTP));

        let server_url = format!("http://{hostname_str}").parse().unwrap();

        let user = CoreUser::new(user_id.clone(), server_url, grpc_port, db_dir, None)
            .await
            .unwrap();
        Self {
            user,
            db_dir: Some(db_dir.to_owned()),
            user_handle_record: None,
        }
    }

    pub fn user(&self) -> &CoreUser {
        &self.user
    }

    pub async fn add_user_handle(&mut self) -> anyhow::Result<UserHandleRecord> {
        if let Some(record) = self.user_handle_record.clone() {
            info!(user_id = ?self.user.user_id(), "User handle already exists");
            return Ok(record);
        }

        let user_id_str = format!("{:?}", self.user.user_id())
            .replace('-', "")
            .replace(['@', '.'], "_");
        let handle = UserHandle::new(user_id_str)?;
        info!(
            user_id = ?self.user.user_id(),
            handle = handle.plaintext(),
            "Adding handle to user"
        );
        let record = self
            .user
            .add_user_handle(&handle)
            .await?
            .context("user handle is already in use")?;
        self.user_handle_record = Some(record.clone());

        Ok(record)
    }
}

enum TestKind {
    SingleBackend(String), // url of the single backend
    Federated,
}

pub struct TestBackend {
    pub users: HashMap<UserId, TestUser>,
    pub groups: HashMap<ConversationId, HashSet<UserId>>,
    // This is what we feed to the test clients.
    kind: TestKind,
    grpc_port: u16,
    temp_dir: TempDir,
    _guard: Option<LocalEnterGuard>,
}

impl TestBackend {
    pub async fn single() -> Self {
        Self::single_with_rate_limits(TEST_RATE_LIMITS).await
    }

    pub async fn single_with_rate_limits(rate_limits: RateLimitsConfig) -> Self {
        let network_provider = MockNetworkProvider::new();
        let domain: Fqdn = "example.com".parse().unwrap();
        let local = LocalSet::new();
        let _guard = local.enter();
        let (addr, _dispatch) =
            spawn_app_with_rate_limits(domain.clone(), network_provider, rate_limits).await;
        info!(%addr, "spawned server");
        Self {
            users: HashMap::new(),
            groups: HashMap::new(),
            kind: TestKind::SingleBackend(addr.to_string()),
            grpc_port: addr.port(),
            temp_dir: tempfile::tempdir().unwrap(),
            _guard: Some(_guard),
        }
    }

    pub fn url(&self) -> Option<String> {
        if let TestKind::SingleBackend(url) = &self.kind {
            Some(url.clone())
        } else {
            None
        }
    }

    pub fn grpc_port(&self) -> u16 {
        self.grpc_port
    }

    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    pub async fn add_persisted_user(&mut self, user_id: &UserId) {
        let path = self.temp_dir.path().to_str().unwrap();
        info!(%path, ?user_id, "Creating persisted user");
        let user = TestUser::new_persisted(user_id, self.url(), self.grpc_port, path).await;
        self.users.insert(user_id.clone(), user);
    }

    pub async fn add_user(&mut self, user_id: &UserId) {
        info!(?user_id, "Creating user");
        let user = TestUser::new(user_id, self.url(), self.grpc_port).await;
        self.users.insert(user_id.clone(), user);
    }

    pub fn get_user(&self, user_id: &UserId) -> &TestUser {
        self.users.get(user_id).unwrap()
    }

    pub fn take_user(&mut self, user_id: &UserId) -> TestUser {
        self.users.remove(user_id).unwrap()
    }

    pub async fn delete_user(&mut self, user_id: &UserId) {
        let test_user = self.take_user(user_id);
        match test_user.db_dir {
            Some(db_dir) => test_user.user.delete(db_dir.as_str()).await.unwrap(),
            None => test_user.user.delete_ephemeral().await.unwrap(),
        }
    }

    /// This has the updater commit an update, but without the checks ensuring
    /// that the group state remains unchanged.
    pub async fn commit_to_proposals(
        &mut self,
        conversation_id: ConversationId,
        updater_id: UserId,
    ) {
        info!(
            "{updater_id:?} performs an update in group {}",
            conversation_id.uuid()
        );

        let test_updater = self.users.get_mut(&updater_id).unwrap();
        let updater = &mut test_updater.user;

        let pending_removes =
            HashSet::from_iter(updater.pending_removes(conversation_id).await.unwrap());
        let group_members_before = updater
            .mls_conversation_participants(conversation_id)
            .await
            .unwrap();

        updater.update_key(conversation_id).await.unwrap();

        let group_members_after = updater
            .mls_conversation_participants(conversation_id)
            .await
            .unwrap();
        let difference: HashSet<UserId> = group_members_before
            .difference(&group_members_after)
            .map(|s| s.to_owned())
            .collect();
        assert_eq!(difference, pending_removes);

        let group_members = self.groups.get(&conversation_id).unwrap();
        // Have all group members fetch and process messages.
        for group_member_id in group_members.iter() {
            // skip the sender
            if group_member_id == &updater_id {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_id).unwrap();
            let group_member = &mut test_group_member.user;
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let pending_removes =
                HashSet::from_iter(group_member.pending_removes(conversation_id).await.unwrap());
            let group_members_before = group_member
                .mls_conversation_participants(conversation_id)
                .await
                .unwrap();

            group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            // If the group member in question is removed with this commit,
            // it should turn its conversation inactive ...
            if pending_removes.contains(group_member_id) {
                let conversation_after = group_member.conversation(&conversation_id).await.unwrap();
                assert!(matches!(&conversation_after.status(),
                ConversationStatus::Inactive(ic)
                if HashSet::from_iter(ic.past_members().to_vec()) ==
                    group_members_before
                ));
            } else {
                // ... if not, it should remove the members to be removed.
                let group_members_after = group_member
                    .mls_conversation_participants(conversation_id)
                    .await
                    .unwrap();
                let difference: HashSet<UserId> = group_members_before
                    .difference(&group_members_after)
                    .cloned()
                    .collect();
                assert_eq!(difference, pending_removes);
            }
        }
    }

    pub async fn update_group(&mut self, conversation_id: ConversationId, updater_id: &UserId) {
        info!(
            "{updater_id:?} performs an update in group {}",
            conversation_id.uuid()
        );

        let test_updater = self.users.get_mut(updater_id).unwrap();
        let updater = &mut test_updater.user;

        updater.update_key(conversation_id).await.unwrap();

        let group_members = self.groups.get(&conversation_id).unwrap();
        // Have all group members fetch and process messages.
        for group_member_id in group_members.iter() {
            // skip the sender
            if group_member_id == updater_id {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_id).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_members_after = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            assert_eq!(group_members_after, group_members_before);
        }
    }

    pub async fn connect_users(&mut self, user1_id: &UserId, user2_id: &UserId) -> ConversationId {
        info!("Connecting users {user1_id:?} and {user2_id:?}");

        let test_user2 = self.users.get_mut(user2_id).unwrap();
        let user2_handle_record = test_user2.add_user_handle().await.unwrap();
        let user2_handle = &user2_handle_record.handle;

        let test_user1 = self.users.get_mut(user1_id).unwrap();
        let user1 = &mut test_user1.user;
        let user1_profile = user1.own_user_profile().await.unwrap();
        let user1_handle_contacts_before = user1.handle_contacts().await.unwrap();
        let user1_conversations_before = user1.conversations().await.unwrap();
        user1.add_contact(user2_handle.clone()).await.unwrap();
        let mut user1_handle_contacts_after = user1.handle_contacts().await.unwrap();
        let error_msg = format!(
            "User 2 should be in the handle contacts list of user 1. List: {user1_handle_contacts_after:?}",
        );
        let new_user_position = user1_handle_contacts_after
            .iter()
            .position(|c| &c.handle == user2_handle)
            .expect(&error_msg);
        // If we remove the new user, the handle contact lists should be the same.
        user1_handle_contacts_after.remove(new_user_position);
        user1_handle_contacts_before
            .into_iter()
            .zip(user1_handle_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.handle, after.handle);
            });
        let mut user1_conversations_after = user1.conversations().await.unwrap();
        let test_title = format!("Connection group: {}", user2_handle.plaintext());
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| c.attributes().title() == test_title)
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type()
                == &ConversationType::HandleConnection(user2_handle.clone())
        );
        user1_conversations_before
            .into_iter()
            .zip(user1_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        let user1_conversation_id = conversation.id();

        let test_user2 = self.users.get_mut(user2_id).unwrap();
        let user2 = &mut test_user2.user;
        let user2_contacts_before = user2.contacts().await.unwrap();
        let user2_conversations_before = user2.conversations().await.unwrap();
        info!("{user2_id:?} fetches and process AS handle messages");
        let (mut stream, responder) = user2.listen_handle(&user2_handle_record).await.unwrap();
        while let Some(Some(message)) = timeout(Duration::from_millis(500), stream.next())
            .await
            .unwrap()
        {
            let message_id = message.message_id.unwrap();
            user2
                .process_handle_queue_message(&user2_handle_record.handle, message)
                .await
                .unwrap();
            responder.ack(message_id.into()).await;
        }

        // User 2 should have auto-accepted (for now at least) the connection request.
        let mut user2_contacts_after = user2.contacts().await.unwrap();
        info!("User 2 contacts after: {:?}", user2_contacts_after);
        let user2_handle_contacts_before = user2.handle_contacts().await.unwrap();
        info!(
            "User 2 handle contacts after: {:?}",
            user2_handle_contacts_before
        );
        let new_contact_position = user2_contacts_after
            .iter()
            .position(|c| &c.user_id == user1_id)
            .expect("User 1 should be in the handle contacts list of user 2");
        // If we remove the new user, the handle contact lists should be the same.
        user2_contacts_after.remove(new_contact_position);
        user2_contacts_before
            .into_iter()
            .zip(user2_contacts_after)
            .for_each(|(before, after)| {
                assert_eq!(before.user_id, after.user_id);
            });
        // User 2 should have created a connection group.
        let mut user2_conversations_after = user2.conversations().await.unwrap();
        info!(
            "User 2 conversations after: {:?}",
            user2_conversations_after
        );
        let new_conversation_position = user2_conversations_after
            .iter()
            .position(|c| {
                c.attributes().title() == user1_profile.display_name.clone().into_string()
            })
            .expect("User 2 should have created a new conversation");
        let conversation = user2_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type() == &ConversationType::Connection(user1_id.clone())
        );
        user2_conversations_before
            .into_iter()
            .zip(user2_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        let user2_conversation_id = conversation.id();

        let user2_id = user2.user_id().clone();
        let test_user1 = self.users.get_mut(user1_id).unwrap();
        let user1 = &mut test_user1.user;
        let user1_contacts_before: HashSet<_> = user1
            .contacts()
            .await
            .unwrap()
            .into_iter()
            .map(|contact| contact.user_id.clone())
            .collect();
        let user1_conversations_before = user1.conversations().await.unwrap();
        info!("{user1_id:?} fetches QS messages");
        let qs_messages = user1.qs_fetch_messages().await.unwrap();
        info!("{user1_id:?} processes QS messages");
        user1.fully_process_qs_messages(qs_messages).await.unwrap();

        // User 1 should have added user 2 to its contacts now and a connection
        // group should have been created.
        let user1_contacts_after: HashSet<_> = user1
            .contacts()
            .await
            .unwrap()
            .into_iter()
            .map(|contact| contact.user_id.clone())
            .collect();
        let new_user_vec: Vec<_> = user1_contacts_after
            .difference(&user1_contacts_before)
            .collect();
        assert_eq!(new_user_vec, vec![&user2_id]);
        // User 2 should have created a connection group.
        let mut user1_conversations_after = user1.conversations().await.unwrap();
        let new_conversation_position = user1_conversations_after
            .iter()
            .position(|c| c.attributes().title() == test_title)
            .expect("User 1 should have created a new conversation");
        let conversation = user1_conversations_after.remove(new_conversation_position);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(
            conversation.conversation_type() == &ConversationType::Connection(user2_id.clone())
        );
        let ids_before: HashSet<_> = user1_conversations_before.iter().map(|c| c.id()).collect();
        let ids_after: HashSet<_> = user1_conversations_after.iter().map(|c| c.id()).collect();
        assert!(ids_before.is_superset(&ids_after));
        debug_assert_eq!(user1_conversation_id, user2_conversation_id);

        let user1_unread_messages = self
            .users
            .get_mut(user1_id)
            .unwrap()
            .user
            .unread_messages_count(user1_conversation_id)
            .await;
        assert_eq!(user1_unread_messages, 0);

        // Send messages both ways to ensure it works.
        self.send_message(user1_conversation_id, user1_id, vec![&user2_id])
            .await;

        let user1_unread_messages = self
            .users
            .get_mut(user1_id)
            .unwrap()
            .user
            .unread_messages_count(user1_conversation_id)
            .await;
        assert_eq!(user1_unread_messages, 0);

        self.send_message(user1_conversation_id, &user2_id, vec![user1_id])
            .await;

        let user1_unread_messages = self
            .users
            .get_mut(user1_id)
            .unwrap()
            .user
            .unread_messages_count(user1_conversation_id)
            .await;
        assert_eq!(user1_unread_messages, 1);

        // Fetch the last message and mark it as read.
        let test_user1 = self.users.get_mut(user1_id).unwrap();
        let user1 = &mut test_user1.user;
        let user1_messages = user1.messages(user1_conversation_id, 1).await.unwrap();

        assert_eq!(user1_messages.len(), 1);
        let user1_unread_messages = user1.unread_messages_count(user1_conversation_id).await;
        assert_eq!(user1_unread_messages, 1);

        let last_message = user1_messages.last().unwrap();

        user1
            .mark_as_read([(user1_conversation_id, last_message.timestamp())].into_iter())
            .await
            .unwrap();

        let user1_unread_messages = user1.unread_messages_count(user1_conversation_id).await;
        assert_eq!(user1_unread_messages, 0);

        let test_user2 = self.users.get_mut(&user2_id).unwrap();
        let user2 = &mut test_user2.user;
        let user2_messages = user2.messages(user2_conversation_id, 1).await.unwrap();

        assert_eq!(user2_messages.len(), 1);
        let last_message = user2_messages.last().unwrap();
        user2
            .mark_as_read([(user2_conversation_id, last_message.timestamp())].into_iter())
            .await
            .unwrap();

        let user2_unread_messages = user2.unread_messages_count(user2_conversation_id).await;
        assert_eq!(user2_unread_messages, 0);

        let member_set: HashSet<UserId> = [user1_id.clone(), user2_id.clone()].into();
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
        sender_id: &UserId,
        recipients: Vec<&UserId>,
    ) -> ConversationMessageId {
        let recipient_strings = recipients
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>();
        info!(
            "{sender_id:?} sends a message to {}",
            recipient_strings.join(", ")
        );
        let message: String = OsRng
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let salt: [u8; 16] = RustCrypto::default().random_array().unwrap();
        let orig_message = MimiContent::simple_markdown_message(message, salt);
        let test_sender = self.users.get_mut(sender_id).unwrap();
        let sender = &mut test_sender.user;

        // Before sending a message, the sender must first fetch and process its QS messages.

        let sender_qs_messages = sender.qs_fetch_messages().await.unwrap();

        sender
            .fully_process_qs_messages(sender_qs_messages)
            .await
            .unwrap();

        let message = test_sender
            .user
            .send_message(conversation_id, orig_message.clone(), None)
            .await
            .unwrap();
        let sender_user_id = test_sender.user.user_id().clone();

        let conversation = test_sender
            .user
            .conversation(&conversation_id)
            .await
            .unwrap();
        let group_id = conversation.group_id();

        assert_eq!(
            message.message(),
            &Message::Content(Box::new(ContentMessage::new(
                test_sender.user.user_id().clone(),
                true,
                orig_message.clone(),
                group_id,
            )))
        );

        for recipient_id in &recipients {
            let recipient = self.users.get_mut(recipient_id).unwrap();
            let recipient_user = &mut recipient.user;

            let recipient_qs_messages = recipient_user.qs_fetch_messages().await.unwrap();

            let messages = recipient_user
                .fully_process_qs_messages(recipient_qs_messages)
                .await
                .unwrap();

            let message = messages.new_messages.last().unwrap();
            let conversaion = recipient_user
                .conversation(&message.conversation_id())
                .await
                .unwrap();
            let group_id = conversaion.group_id();

            assert_eq!(
                message.message(),
                &Message::Content(Box::new(ContentMessage::new(
                    sender_user_id.clone(),
                    true,
                    orig_message.clone(),
                    group_id
                )))
            );
        }
        message.id()
    }

    pub async fn send_attachment(
        &mut self,
        conversation_id: ConversationId,
        sender_id: &UserId,
        recipients: Vec<&UserId>,
        attachment: &[u8],
        filename: &str,
    ) -> (ConversationMessageId, NestedPartContent) {
        let recipient_strings = recipients
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>();
        info!(
            "{sender_id:?} sends a message to {}",
            recipient_strings.join(", ")
        );

        let test_sender = self.users.get_mut(sender_id).unwrap();
        let sender = &mut test_sender.user;

        // Before sending a message, the sender must first fetch and process its QS messages.
        let sender_qs_messages = sender.qs_fetch_messages().await.unwrap();
        sender
            .fully_process_qs_messages(sender_qs_messages)
            .await
            .unwrap();

        let tmp_dir = TempDir::new().unwrap();
        let path = tmp_dir.path().join(filename);
        std::fs::write(&path, attachment).unwrap();

        let message = sender
            .upload_attachment(conversation_id, &path)
            .await
            .unwrap();

        let mut external_part = None;
        message
            .message()
            .mimi_content()
            .unwrap()
            .visit_attachments(|part| {
                assert!(external_part.replace(part.clone()).is_none());
                Ok(())
            })
            .unwrap();
        let external_part = external_part.unwrap();
        match &external_part {
            NestedPartContent::ExternalPart {
                enc_alg,
                key,
                nonce,
                hash_alg,
                ..
            } => {
                assert_eq!(*enc_alg, EncryptionAlgorithm::Aes256Gcm12);
                assert_eq!(nonce.len(), 12);
                assert_eq!(key.len(), 32);
                assert_eq!(*hash_alg, HashAlgorithm::Sha256);
            }
            _ => panic!("unexpected attachment type"),
        };

        for recipient_id in &recipients {
            let recipient = self.users.get_mut(recipient_id).unwrap();
            let recipient_user = &mut recipient.user;

            let recipient_qs_messages = recipient_user.qs_fetch_messages().await.unwrap();
            let messages = recipient_user
                .fully_process_qs_messages(recipient_qs_messages)
                .await
                .unwrap();

            let mut attachment_found_once = false;
            messages
                .new_messages
                .last()
                .unwrap()
                .message()
                .mimi_content()
                .unwrap()
                .visit_attachments(|part| {
                    assert!(!attachment_found_once);

                    // Removed cleared fields from the expected attachment.
                    let mut expected = external_part.clone();
                    if let NestedPartContent::ExternalPart {
                        key,
                        nonce,
                        aad,
                        content_hash,
                        ..
                    } = &mut expected
                    {
                        key.clear();
                        nonce.clear();
                        aad.clear();
                        content_hash.clear();
                    } else {
                        panic!("Unexpected attachment type")
                    }

                    assert_eq!(part, &expected);
                    attachment_found_once = true;
                    Ok(())
                })
                .unwrap();
        }

        (message.id(), external_part)
    }

    pub async fn create_group(&mut self, user_id: &UserId) -> ConversationId {
        let test_user = self.users.get_mut(user_id).unwrap();
        let user = &mut test_user.user;
        let user_conversations_before = user.conversations().await.unwrap();

        let group_name = format!("{:?}", OsRng.r#gen::<[u8; 32]>());
        let group_picture_bytes_option = Some(OsRng.r#gen::<[u8; 32]>().to_vec());
        let conversation_id = user
            .create_conversation(group_name.clone(), group_picture_bytes_option.clone())
            .await
            .unwrap();
        let mut user_conversations_after = user.conversations().await.unwrap();
        let new_conversation_position = user_conversations_after
            .iter()
            .position(|c| c.attributes().title() == group_name)
            .expect("User 1 should have created a new conversation");
        let conversation = user_conversations_after.remove(new_conversation_position);
        assert!(conversation.id() == conversation_id);
        assert!(conversation.status() == &ConversationStatus::Active);
        assert!(conversation.conversation_type() == &ConversationType::Group);
        assert_eq!(conversation.attributes().title(), &group_name);
        assert_eq!(
            conversation.attributes().picture(),
            group_picture_bytes_option.as_deref()
        );
        user_conversations_before
            .into_iter()
            .zip(user_conversations_after)
            .for_each(|(before, after)| {
                assert_eq!(before.id(), after.id());
            });
        let member_set: HashSet<UserId> = [user_id.clone()].into();
        assert_eq!(member_set.len(), 1);
        self.groups.insert(conversation_id, member_set);

        conversation_id
    }

    /// Has the inviter invite the invitees to the given group and has everyone
    /// send and process their messages.
    pub async fn invite_to_group(
        &mut self,
        conversation_id: ConversationId,
        inviter_id: &UserId,
        invitees: Vec<&UserId>,
    ) {
        let invitee_strings = invitees
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>();
        let test_inviter = self.users.get_mut(inviter_id).unwrap();
        let inviter = &mut test_inviter.user;

        // Before inviting anyone to a group, the inviter must first fetch and
        // process its QS messages.
        let qs_messages = inviter.qs_fetch_messages().await.unwrap();

        inviter
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");
        let inviter_conversation = inviter.conversation(&conversation_id).await.unwrap();

        info!(
            "{inviter_id:?} invites {} to the group with id {}",
            invitee_strings.join(", "),
            conversation_id.uuid()
        );

        // Perform the invite operation and check that the invitees are now in the group.
        let inviter_group_members_before = inviter
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");

        let invite_messages = inviter
            .invite_users(
                conversation_id,
                &invitees.iter().cloned().cloned().collect::<Vec<_>>(),
            )
            .await
            .expect("Error inviting users.");

        let mut expected_messages = HashSet::new();
        for invitee_id in &invitees {
            let expected_message =
                format!("{inviter_id:?} added {invitee_id:?} to the conversation");
            expected_messages.insert(expected_message);
        }

        let invite_messages = display_messages_to_string_map(invite_messages);

        assert_eq!(invite_messages, expected_messages);

        let inviter_group_members_after = inviter
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");
        let new_members = inviter_group_members_after
            .difference(&inviter_group_members_before)
            .collect::<HashSet<_>>();
        let invitee_set = invitees.iter().copied().collect::<HashSet<_>>();
        assert_eq!(new_members, invitee_set);

        // Now that the invitation is out, have the invitees and all other group
        // members fetch and process QS messages.
        for invitee_id in &invitees {
            let test_invitee = self.users.get_mut(invitee_id).unwrap();
            let invitee = &mut test_invitee.user;
            let mut invitee_conversations_before = invitee.conversations().await.unwrap();

            let qs_messages = invitee.qs_fetch_messages().await.unwrap();

            invitee
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let mut invitee_conversations_after = invitee.conversations().await.unwrap();
            let conversation_uuid = conversation_id.uuid();
            let new_conversation_position = invitee_conversations_after
                .iter()
                .position(|c| c.id() == conversation_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{invitee_id:?} should have created a new conversation {conversation_uuid}"
                    )
                });
            let conversation = invitee_conversations_after.remove(new_conversation_position);
            assert!(conversation.id() == conversation_id);
            assert!(conversation.status() == &ConversationStatus::Active);
            assert!(conversation.conversation_type() == &ConversationType::Group);
            assert_eq!(
                conversation.attributes().title(),
                inviter_conversation.attributes().title()
            );
            assert_eq!(
                conversation.attributes().picture(),
                inviter_conversation.attributes().picture()
            );
            // In case it was a re-join, we remove it from the conversation list before as well.
            if let Some(inactive_conversation_position) = invitee_conversations_before
                .iter()
                .position(|c| c.id() == conversation_id)
            {
                invitee_conversations_before.remove(inactive_conversation_position);
            }
            // Now that we've removed the new conversation, it should be the same set of conversations
            info!("Conversations_before: {:?}", invitee_conversations_before);
            info!("Conversations_after: {:?}", invitee_conversations_after);
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
        }
        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        for group_member_id in group_members.iter() {
            // Skip the sender
            if group_member_id == inviter_id {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_id).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let invite_messages = group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let invite_messages = display_messages_to_string_map(invite_messages.new_messages);

            assert_eq!(invite_messages, expected_messages);

            let group_members_after = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let new_members = group_members_after
                .difference(&group_members_before)
                .collect::<HashSet<_>>();
            let invitee_set = invitees.iter().copied().collect::<HashSet<_>>();
            assert_eq!(new_members, invitee_set)
        }

        for invitee_id in &invitees {
            let unique_member = group_members.insert((*invitee_id).clone());
            assert!(unique_member);
        }

        // Now send messages to check that the group works properly. This also
        // ensures that everyone involved has picked up their messages from the
        // QS and that notifications are flushed.
        self.send_message(conversation_id, inviter_id, invitees.clone())
            .await;
        for invitee_id in &invitees {
            let recipients: Vec<_> = invitees
                .iter()
                .filter(|&name| name != invitee_id)
                .chain([&inviter_id].into_iter())
                .cloned()
                .collect();
            self.send_message(conversation_id, invitee_id, recipients)
                .await;
        }
    }

    /// Has the remover remove the removed from the given group and has everyone
    /// send and process their messages.
    pub async fn remove_from_group(
        &mut self,
        conversation_id: ConversationId,
        remover_id: &UserId,
        removed_ids: Vec<&UserId>,
    ) -> anyhow::Result<()> {
        let removed_strings = removed_ids
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>();
        let test_remover = self.users.get_mut(remover_id).unwrap();
        let remover = &mut test_remover.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = remover.qs_fetch_messages().await.unwrap();

        remover
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        info!(
            "{remover_id:?} removes {} from the group with id {}",
            removed_strings.join(", "),
            conversation_id.uuid()
        );

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let remover_group_members_before = remover
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");

        let remove_messages = remover
            .remove_users(
                conversation_id,
                removed_ids.iter().copied().cloned().collect::<Vec<_>>(),
            )
            .await?;

        let mut expected_messages = HashSet::new();

        for removed_id in &removed_ids {
            let expected_message =
                format!("{remover_id:?} removed {removed_id:?} from the conversation");
            expected_messages.insert(expected_message);
        }

        let remove_messages = display_messages_to_string_map(remove_messages);
        assert_eq!(remove_messages, expected_messages);

        let remover_group_members_after = remover
            .conversation_participants(conversation_id)
            .await
            .expect("Error getting group members.");
        let removed_members: HashSet<_> = remover_group_members_before
            .difference(&remover_group_members_after)
            .cloned()
            .collect();
        let removed_set: HashSet<_> = removed_ids.iter().map(|&user_id| user_id.clone()).collect();
        assert_eq!(removed_members, removed_set);

        for removed_id in &removed_ids {
            let test_removed = self.users.get_mut(removed_id).unwrap();
            let removed = &mut test_removed.user;
            let removed_conversations_before = removed
                .conversations()
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();
            let past_members = removed
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            let qs_messages = removed.qs_fetch_messages().await.unwrap();

            removed
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let removed_conversations_after = removed
                .conversations()
                .await
                .unwrap()
                .into_iter()
                .collect::<HashSet<_>>();
            let conversation = removed_conversations_after
                .iter()
                .find(|c| c.id() == conversation_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{removed_id:?} should have the conversation with id {}",
                        conversation_id.uuid()
                    )
                });
            assert!(conversation.id() == conversation_id);
            if let ConversationStatus::Inactive(inactive_status) = &conversation.status() {
                let inactive_status_members =
                    HashSet::from_iter(inactive_status.past_members().to_vec());
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
            assert!(conversation.conversation_type() == &ConversationType::Group);
            for conversation in removed_conversations_after {
                assert!(
                    removed_conversations_before
                        .iter()
                        .any(|c| c.id() == conversation.id())
                )
            }
        }
        let group_members = self.groups.get_mut(&conversation_id).unwrap();
        for removed_id in &removed_ids {
            let remove_successful = group_members.remove(removed_id);
            assert!(remove_successful);
        }
        // Now have the rest of the group pick up and process their messages.
        for group_member_id in group_members.iter() {
            // Skip the remover
            if group_member_id == remover_id {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_id).unwrap();
            let group_member = &mut test_group_member.user;
            let group_members_before = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            let remove_messages = group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let remove_messages = display_messages_to_string_map(remove_messages.new_messages);
            assert_eq!(remove_messages, expected_messages);

            let group_members_after = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();
            let removed_members: HashSet<_> = group_members_before
                .difference(&group_members_after)
                .cloned()
                .collect();
            let removed_set: HashSet<_> = removed_ids.iter().cloned().cloned().collect();
            assert_eq!(removed_members, removed_set)
        }

        Ok(())
    }

    /// Has the leaver leave the given group.
    pub async fn leave_group(
        &mut self,
        conversation_id: ConversationId,
        leaver_id: &UserId,
    ) -> anyhow::Result<()> {
        info!(
            "{leaver_id:?} leaves the group with id {}",
            conversation_id.uuid()
        );

        // Get random member that observes the proposal and can commit it later
        let group_members = self.groups.get(&conversation_id).unwrap().clone();
        let mut random_member_iter = group_members.iter();
        let mut random_member_id = random_member_iter.next().unwrap();
        // Ensure that the random member isn't the leaver.
        if random_member_id == leaver_id {
            random_member_id = random_member_iter.next().unwrap()
        }

        // Perform the leave operation.
        let test_random_member = self.users.get_mut(random_member_id).unwrap();
        let random_member = &mut test_random_member.user;
        let mimi_members_before = random_member
            .conversation_participants(conversation_id)
            .await
            .unwrap();
        let test_leaver = self.users.get_mut(leaver_id).unwrap();
        let leaver = &mut test_leaver.user;
        leaver.leave_conversation(conversation_id).await?;

        // Fetch and process the QS messages to make sure the random member has the proposal.
        let test_random_member = self.users.get_mut(random_member_id).unwrap();
        let random_member = &mut test_random_member.user;
        let qs_messages = random_member.qs_fetch_messages().await.unwrap();

        random_member
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        let mimi_members_after = random_member
            .conversation_participants(conversation_id)
            .await
            .unwrap();
        let difference: HashSet<UserId> = mimi_members_before
            .difference(&mimi_members_after)
            .map(|s| s.to_owned())
            .collect();
        let pending_removes = HashSet::from_iter(
            random_member
                .pending_removes(conversation_id)
                .await
                .unwrap(),
        );
        assert_eq!(difference, pending_removes);

        // Now have a random group member perform an update, thus committing the leave operation.
        // TODO: This is not really random. We should do better here. But also,
        // we probably want a way to track the randomness s.t. we can reproduce
        // tests.

        // Now commit to the pending proposal. This also makes everyone else
        // pick up and process their messages. This also tests that group
        // members were removed correctly from the local group and that the
        // leaver has turned its conversation inactive.
        self.commit_to_proposals(conversation_id, random_member_id.clone())
            .await;

        let group_members = self.groups.get_mut(&conversation_id).unwrap();

        group_members.remove(leaver_id);

        Ok(())
    }

    pub async fn delete_group(&mut self, conversation_id: ConversationId, deleter_id: &UserId) {
        info!(
            "{deleter_id:?} deletes the group with id {}",
            conversation_id.uuid()
        );
        let test_deleter = self.users.get_mut(deleter_id).unwrap();
        let deleter = &mut test_deleter.user;

        // Before removing anyone from a group, the remover must first fetch and
        // process its QS messages.
        let qs_messages = deleter.qs_fetch_messages().await.unwrap();

        deleter
            .fully_process_qs_messages(qs_messages)
            .await
            .expect("Error processing qs messages.");

        // Perform the remove operation and check that the removed are not in
        // the group anymore.
        let deleter_conversation_before = deleter
            .conversation(&conversation_id)
            .await
            .unwrap()
            .clone();
        assert_eq!(
            deleter_conversation_before.status(),
            &ConversationStatus::Active
        );
        let past_members = deleter
            .conversation_participants(conversation_id)
            .await
            .unwrap();

        deleter.delete_conversation(conversation_id).await.unwrap();

        let deleter_conversation_after = deleter.conversation(&conversation_id).await.unwrap();
        if let ConversationStatus::Inactive(inactive_status) = &deleter_conversation_after.status()
        {
            let inactive_status_members =
                HashSet::from_iter(inactive_status.past_members().to_vec());
            assert_eq!(inactive_status_members, past_members);
        } else {
            panic!("Conversation should be inactive.")
        }

        for group_member_id in self.groups.get(&conversation_id).unwrap().iter() {
            // Skip the deleter
            if group_member_id == deleter_id {
                continue;
            }
            let test_group_member = self.users.get_mut(group_member_id).unwrap();
            let group_member = &mut test_group_member.user;

            let group_member_conversation_before =
                group_member.conversation(&conversation_id).await.unwrap();
            assert_eq!(
                group_member_conversation_before.status(),
                &ConversationStatus::Active
            );
            let past_members = group_member
                .conversation_participants(conversation_id)
                .await
                .unwrap();

            let qs_messages = group_member.qs_fetch_messages().await.unwrap();

            group_member
                .fully_process_qs_messages(qs_messages)
                .await
                .expect("Error processing qs messages.");

            let group_member_conversation_after =
                group_member.conversation(&conversation_id).await.unwrap();
            if let ConversationStatus::Inactive(inactive_status) =
                &group_member_conversation_after.status()
            {
                let inactive_status_members =
                    HashSet::from_iter(inactive_status.past_members().to_vec());
                assert_eq!(inactive_status_members, past_members);
            } else {
                panic!("Conversation should be inactive.")
            }
        }
        self.groups.remove(&conversation_id);
    }

    pub fn random_user(&self, rng: &mut impl RngCore) -> UserId {
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
                let mut other_users = Vec::new();
                for user in self.users.keys() {
                    let is_contact = self
                        .users
                        .get(user)
                        .unwrap()
                        .user()
                        .contacts()
                        .await
                        .unwrap()
                        .into_iter()
                        .any(|contact| contact.user_id == random_user);
                    if user != &random_user && !is_contact {
                        other_users.push(user.clone());
                    }
                }
                if let Some(other_user) = other_users.into_iter().choose(rng) {
                    info!(
                        random_operation = true,
                        "Random operation: Connecting {random_user:?} and {other_user:?}"
                    );
                    self.connect_users(&random_user, &other_user).await;
                }
            }
            1 => {
                let conversation_id = self.create_group(&random_user).await;
                info!(
                    random_operation = true,
                    "Random operation: Created group {}",
                    conversation_id.uuid()
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
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
                    })
                    .choose(rng)
                {
                    let number_of_invitees = rng.gen_range(1..=5);
                    let mut invitees = Vec::new();
                    for invitee in self.users.keys() {
                        let is_group_member = self
                            .groups
                            .get(&conversation.id())
                            .unwrap()
                            .contains(invitee);
                        let is_connected = user
                            .user()
                            .contacts()
                            .await
                            .unwrap()
                            .into_iter()
                            .any(|contact| &contact.user_id == invitee);
                        if !is_group_member && is_connected && invitee != &random_user {
                            invitees.push(invitee);
                        }
                    }
                    let invitees = invitees
                        .into_iter()
                        .cloned()
                        .choose_multiple(rng, number_of_invitees);
                    // It can happen that there are no suitable users to invite
                    if !invitees.is_empty() {
                        let invitee_strings = invitees
                            .iter()
                            .map(|invitee| format!("{invitee:?}"))
                            .collect::<Vec<_>>();
                        info!(
                            random_operation = true,
                            "Random operation: {random_user:?} invites {} to group {}",
                            invitee_strings.join(", "),
                            conversation.id().uuid()
                        );
                        self.invite_to_group(
                            conversation.id(),
                            &random_user,
                            invitees.iter().collect(),
                        )
                        .await;
                    }
                }
            }
            3 => {
                let user = self.users.get(&random_user).unwrap();
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
                    })
                    .choose(rng)
                {
                    let number_of_removals = rng.gen_range(1..=5);
                    let members_to_remove = self
                        .groups
                        .get(&conversation.id())
                        .unwrap()
                        .iter()
                        .filter(|&member| member != &random_user)
                        .cloned()
                        .choose_multiple(rng, number_of_removals);
                    if !members_to_remove.is_empty() {
                        let removed_strings = members_to_remove
                            .iter()
                            .map(|removed| format!("{removed:?}"))
                            .collect::<Vec<_>>();
                        info!(
                            random_operation = true,
                            "Random operation: {random_user:?} removes {} from group {}",
                            removed_strings.join(", "),
                            conversation.id().uuid()
                        );
                        let members_to_remove = members_to_remove.iter().collect();
                        self.remove_from_group(conversation.id(), &random_user, members_to_remove)
                            .await
                            .unwrap();
                    }
                }
            }
            4 => {
                let user = self.users.get(&random_user).unwrap();
                if let Some(conversation) = user
                    .user()
                    .conversations()
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|conversation| {
                        conversation.conversation_type() == &ConversationType::Group
                            && conversation.status() == &ConversationStatus::Active
                    })
                    .choose(rng)
                {
                    info!(
                        random_operation = true,
                        "Random operation: {random_user:?} leaves group {}",
                        conversation.id().uuid()
                    );
                    self.leave_group(conversation.id(), &random_user)
                        .await
                        .unwrap();
                }
            }
            _ => panic!("Invalid action"),
        }
    }
}

fn display_messages_to_string_map(display_messages: Vec<ConversationMessage>) -> HashSet<String> {
    display_messages
        .into_iter()
        .filter_map(|m| {
            if let Message::Event(EventMessage::System(system_message)) = m.message() {
                match system_message {
                    SystemMessage::Add(adder, added) => {
                        Some(format!("{adder:?} added {added:?} to the conversation"))
                    }
                    SystemMessage::Remove(remover, removed) => Some(format!(
                        "{remover:?} removed {removed:?} from the conversation"
                    )),
                }
            } else {
                None
            }
        })
        .collect()
}
