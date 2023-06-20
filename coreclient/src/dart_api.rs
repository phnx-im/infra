// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub use std::sync::Mutex;

use anyhow::Result;
use flutter_rust_bridge::{RustOpaque, StreamSink};

use crate::notifications::Notifiable;
pub use crate::{types::*, Corelib};

#[path = "../dart-bridge/bridge_generated.rs"]
mod bridge_generated;

pub fn _plonk() -> NotificationType {
    NotificationType::ConversationChange
}

#[derive(Clone)]
pub struct DartNotifier {
    pub sink: StreamSink<NotificationType>,
}

impl Notifiable for DartNotifier {
    fn notify(&self, notification_type: NotificationType) -> bool {
        self.sink.add(notification_type)
    }
}

impl From<StreamSink<NotificationType>> for DartNotifier {
    fn from(sink: StreamSink<NotificationType>) -> Self {
        Self { sink }
    }
}

pub struct RustState {
    pub corelib: RustOpaque<Mutex<Corelib<DartNotifier>>>,
}

impl RustState {
    pub fn initialize_backend(&self, url: String) {
        /*  let corelib = &mut self.corelib.lock().unwrap();
        corelib.initialize_backend(&url); */
    }

    pub fn create_user(&self, username: String) -> Result<()> {
        /*  let corelib = &mut self.corelib.lock().unwrap();
        match corelib.create_user(&username) {
            Ok(_) => Ok(()),
            Err(_) => {
                bail!("Failed to create user")
            }
        } */
        todo!()
    }

    pub fn create_conversation(&self, name: String) -> Result<UuidBytes> {
        /*  let corelib = &mut self.corelib.lock().unwrap();
        match corelib.create_conversation(&name) {
            Ok(uuid) => Ok(UuidBytes::from_uuid(&uuid)),
            Err(_) => {
                bail!("Failed to create conversation")
            }
        } */
        todo!()
    }

    pub fn get_conversations(&self) -> Vec<Conversation> {
        /*  let corelib = &mut self.corelib.lock().unwrap();
        corelib.get_conversations() */
        todo!()
    }

    pub fn invite_user(&self, conversation_id: UuidBytes, username: String) -> Result<()> {
        /* let corelib = &mut self.corelib.lock().unwrap();
        match corelib.invite_user(conversation_id.as_uuid(), &username) {
            Ok(_) => Ok(()),
            Err(e) => {
                bail!("Failed to invite user: {}", e)
            }
        } */
        todo!()
    }

    pub fn send_message(
        &self,
        conversation_id: UuidBytes,
        message: String,
    ) -> Result<ConversationMessage> {
        /* let corelib = &mut self.corelib.lock().unwrap();
        match corelib.send_message(conversation_id.as_uuid(), &message) {
            Ok(message) => Ok(message),
            Err(_) => {
                bail!("Failed to send message")
            }
        } */
        todo!()
    }

    pub fn get_messages(
        &self,
        conversation_id: UuidBytes,
        last_n: usize,
    ) -> Vec<ConversationMessage> {
        /*  let corelib = &mut self.corelib.lock().unwrap();
        corelib.get_messages(&conversation_id.as_uuid(), last_n) */
        todo!()
    }

    pub fn get_clients(&self) -> Vec<String> {
        /* let corelib = &mut self.corelib.lock().unwrap();
        corelib
            .list_clients()
            .map(|client| {
                client
                    .iter()
                    .map(|c| c.client_name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default() */
        todo!()
    }

    /*  pub fn register_stream(&self, sink: StreamSink<NotificationType>) -> Result<()> {
        /* let corelib = &mut self.corelib.lock().unwrap();
        corelib
            .notification_hub
            .sinks
            .push(DartNotifier::from(sink).notifier()); */
        Ok(())
    } */

    pub fn fetch_messages(&self) -> Result<()> {
        /*  let corelib = &mut self.corelib.lock().unwrap();
        let backend = if let Some(backend) = &corelib.backend {
            backend
        } else {
            bail!("Backend not initialized");
        };
        let credential = if let Some(user) = &corelib.self_user {
            &user.credential_with_key.credential
        } else {
            bail!("User not created")
        };

        match backend.recv_msgs(credential) {
            Ok(messages) => {
                if messages.is_empty() {
                    print!(".");
                } else {
                    println!("{} new message(s).", messages.len());
                    if let Err(e) = corelib.process_queue_messages(messages.into_vec()) {
                        bail!("Failed to process messages: {e:?}");
                    }
                }
            }
            Err(e) => {
                println!("Error occured when fetching messages from DS: {e:?}");
            }
        }; */
        Ok(())
    }
}

pub fn init_lib() -> RustState {
    RustState {
        corelib: RustOpaque::new(Mutex::new(Corelib::new())),
    }
}
