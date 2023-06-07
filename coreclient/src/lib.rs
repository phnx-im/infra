// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[macro_use]
mod errors;
mod contacts;
mod conversations;
mod groups;
mod notifications;
mod providers;
mod types;
mod users;
mod utils;

#[cfg(feature = "dart-bridge")]
mod dart_api;

use std::collections::HashMap;

pub(crate) use crate::errors::*;
use crate::{conversations::*, groups::*, types::*, users::*};

use notifications::{Notifiable, NotificationHub};
pub(crate) use openmls::prelude::*;
pub(crate) use openmls_rust_crypto::OpenMlsRustCrypto;

use ds_lib::{ClientInfo, GroupMessage};

use uuid::Uuid;

#[derive(Default)]
pub struct Corelib<T>
where
    T: Notifiable,
{
    self_user: Option<SelfUser>,
    notification_hub: NotificationHub<T>,
}

impl<T: Notifiable> Corelib<T> {
    pub fn new() -> Self {
        Self {
            self_user: None,
            notification_hub: NotificationHub::<T>::default(),
        }
    }

    /// Get existing conversations
    pub fn get_conversations(&self) -> Vec<Conversation> {
        match &self.self_user {
            Some(user) => user.conversation_store.conversations(),
            None => vec![],
        }
    }

    pub fn get_messages(&self, conversation_id: &Uuid, last_n: usize) -> Vec<ConversationMessage> {
        match &self.self_user {
            Some(user) => user.conversation_store.messages(conversation_id, last_n),
            None => vec![],
        }
    }

    /// Process the queue messages from the DS
    pub fn process_queue_messages(
        &mut self,
        messages: Vec<MlsMessageIn>,
    ) -> Result<(), CorelibError> {
        let self_user = match &mut self.self_user {
            Some(self_user) => self_user,
            None => return Err(CorelibError::UserNotInitialized),
        };
        let mut group_queues: HashMap<Uuid, Vec<MlsMessageIn>> = HashMap::new();

        for message in messages {
            match message.wire_format() {
                WireFormat::PrivateMessage => {
                    println!("Received a private message");
                    let group_id = UuidBytes::from_bytes(
                        message
                            .clone()
                            .into_protocol_message()
                            .unwrap()
                            .group_id()
                            .as_slice(),
                    )
                    .as_uuid();
                    match group_queues.get_mut(&group_id) {
                        Some(group_queue) => {
                            group_queue.push(message);
                        }
                        None => {
                            group_queues.insert(group_id, vec![message]);
                        }
                    }
                }
                WireFormat::Welcome => {
                    if let Some(welcome) = message.into_welcome() {
                        println!("Received a Welcome message");
                        match Group::join_group(&self_user.crypto_backend, welcome) {
                            Ok(group) => {
                                let group_id = group.group_id();
                                let conversation_id = Uuid::new_v4();
                                match self_user.group_store.store_group(group) {
                                    Ok(()) => {
                                        let attributes = ConversationAttributes {
                                            title: "New conversation".to_string(),
                                        };
                                        self_user.conversation_store.create_group_conversation(
                                            conversation_id,
                                            group_id,
                                            attributes,
                                        );
                                        self.notification_hub.dispatch_conversation_notification();
                                    }
                                    Err(_) => {
                                        println!("Group already exists");
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Could not join group: {:?}", e);
                            }
                        }
                    }
                }
                _ => {
                    println!("Received an unsupported message type");
                }
            }
        }
        for (group_id, group_queue) in group_queues {
            self.process_messages(group_id, group_queue)?;
        }
        Ok(())
    }

    /// Process received messages by group
    pub fn process_messages(
        &mut self,
        group_id: Uuid,
        messages: Vec<MlsMessageIn>,
    ) -> Result<(), CorelibError> {
        let user = match &mut self.self_user {
            Some(user) => user,
            None => return Err(CorelibError::UserNotInitialized),
        };
        let notification_messages = user.process_messages(group_id, messages)?;

        for message in notification_messages {
            self.notification_hub.dispatch_message_notification(message);
        }
        Ok(())
    }
}

// Expose FFI functions
//implement_dart_ffi!(Corelib);

#[test]
fn test_create_user() {
    // TODO: re-enable this when we integrate with the server
    return;

    use rand::prelude::*;

    let username = &format!("unittest_{}", random::<u64>());

    #[derive(Debug, Clone, Default)]
    struct Notifier {}

    impl Notifiable for Notifier {
        fn notify(&self, _notification: NotificationType) -> bool {
            true
        }
    }

    let mut corelib = Corelib::<Notifier>::default();
    corelib.initialize_backend("https://127.0.0.1");
    corelib
        .create_user(username)
        .expect("Could not create user");
}

#[test]
fn test_user_full_cycle() {
    // TODO: re-enable this when we integrate with the server
    return;

    use rand::prelude::*;

    #[derive(Debug, Clone, Default)]
    struct Notifier {}

    impl Notifiable for Notifier {
        fn notify(&self, _notification: NotificationType) -> bool {
            true
        }
    }

    let url = "https://127.0.0.1";
    let rand_str = format!("{}", random::<u64>());
    let alice = &format!("unittest_alice_{}", rand_str);
    let bob = &format!("unittest_bob_{}", rand_str);
    let group_name = "test_conversation";
    let message = "Hello world!";

    // Create user Alice
    let mut alice_corelib = Corelib::<Notifier>::default();
    alice_corelib.initialize_backend(url);
    alice_corelib
        .create_user(alice)
        .expect("Could not create user Alice");

    // Create user Bob
    let mut bob_corelib = Corelib::<Notifier>::default();
    bob_corelib.initialize_backend(url);
    bob_corelib
        .create_user(bob)
        .expect("Could not create user Bob");

    // Alice invites Bob
    println!("Create conversation");
    let group_uuid = alice_corelib
        .create_conversation(group_name)
        .expect("Could not create conversation.");

    println!("Send message in new conversation");
    alice_corelib
        .send_message(group_uuid, message)
        .expect("Could not send application message before invitation:");

    println!("Invite user Bob");
    alice_corelib
        .invite_user(group_uuid, bob)
        .expect("Could not invite user");

    // Alice sends a message
    println!("Send message after Bob joined");
    alice_corelib
        .send_message(group_uuid, message)
        .expect("Could not send application message after invitation:");

    // Bob retrieves messages
    println!("Bob retrieves messages");
    let bob_messages = bob_corelib.get_messages(&group_uuid, 1);

    assert_eq!(bob_messages.len(), 1);
    let bob_message = &bob_messages[0];

    if let Message::Content(bob_content_message) = &bob_message.message {
        if let MessageContentType::Text(text_message) = &bob_content_message.content {
            assert_eq!(&text_message.message, message);
        } else {
            panic!("Wrong content type");
        }
    } else {
        panic!("Wrong message type");
    }
}

#[test]
fn test_list_clients() {
    // TODO: re-enable this when we integrate with the server
    return;

    #[derive(Debug, Clone, Default)]
    struct Notifier {}

    impl Notifiable for Notifier {
        fn notify(&self, _notification: NotificationType) -> bool {
            true
        }
    }

    let mut corelib = Corelib::<Notifier>::default();
    corelib.initialize_backend("https://127.0.0.1");
    let clients = corelib.list_clients().expect("Could not fetch clients");
    println!("Client list:");
    for client in clients {
        println!("\t{}", client.client_name);
    }
}
