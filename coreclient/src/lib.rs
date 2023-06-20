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
pub mod users;
mod utils;

#[cfg(feature = "dart-bridge")]
mod dart_api;

use std::collections::HashMap;

pub(crate) use crate::errors::*;
use crate::{conversations::*, groups::*, types::*, users::*};

use notifications::{Notifiable, NotificationHub};
pub(crate) use openmls::prelude::*;
pub(crate) use openmls_rust_crypto::OpenMlsRustCrypto;

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

    todo!()
}

#[test]
fn test_user_full_cycle() {
    // TODO: re-enable this when we integrate with the server
    return;

    //use rand::prelude::*;

    //#[derive(Debug, Clone, Default)]
    //struct Notifier {}

    //impl Notifiable for Notifier {
    //    fn notify(&self, _notification: NotificationType) -> bool {
    //        true
    //    }
    //}

    //let url = "https://127.0.0.1";
    //let rand_str = format!("{}", random::<u64>());
    //let alice = &format!("unittest_alice_{}", rand_str);
    //let bob = &format!("unittest_bob_{}", rand_str);
    //let group_name = "test_conversation";
    //let message = "Hello world!";

    //// Create user Alice
    //let mut alice_corelib = Corelib::<Notifier>::default();
    //alice_corelib.initialize_backend(url);
    //alice_corelib
    //    .create_user(alice)
    //    .expect("Could not create user Alice");

    //// Create user Bob
    //let mut bob_corelib = Corelib::<Notifier>::default();
    //bob_corelib.initialize_backend(url);
    //bob_corelib
    //    .create_user(bob)
    //    .expect("Could not create user Bob");

    //// Alice invites Bob
    //println!("Create conversation");
    //let group_uuid = alice_corelib
    //    .create_conversation(group_name)
    //    .expect("Could not create conversation.");

    //println!("Send message in new conversation");
    //alice_corelib
    //    .send_message(group_uuid, message)
    //    .expect("Could not send application message before invitation:");

    //println!("Invite user Bob");
    //alice_corelib
    //    .invite_user(group_uuid, bob)
    //    .expect("Could not invite user");

    //// Alice sends a message
    //println!("Send message after Bob joined");
    //alice_corelib
    //    .send_message(group_uuid, message)
    //    .expect("Could not send application message after invitation:");

    //// Bob retrieves messages
    //println!("Bob retrieves messages");
    //let bob_messages = bob_corelib.get_messages(&group_uuid, 1);

    //assert_eq!(bob_messages.len(), 1);
    //let bob_message = &bob_messages[0];

    //if let Message::Content(bob_content_message) = &bob_message.message {
    //    if let MessageContentType::Text(text_message) = &bob_content_message.content {
    //        assert_eq!(&text_message.message, message);
    //    } else {
    //        panic!("Wrong content type");
    //    }
    //} else {
    //    panic!("Wrong message type");
    //}
}

#[test]
fn test_list_clients() {
    // TODO: re-enable this when we integrate with the server
    return;

    //#[derive(Debug, Clone, Default)]
    //struct Notifier {}

    //impl Notifiable for Notifier {
    //    fn notify(&self, _notification: NotificationType) -> bool {
    //        true
    //    }
    //}

    //let mut corelib = Corelib::<Notifier>::default();
    //corelib.initialize_backend("https://127.0.0.1");
    //let clients = corelib.list_clients().expect("Could not fetch clients");
    //println!("Client list:");
    //for client in clients {
    //    println!("\t{}", client.client_name);
    //}
}
