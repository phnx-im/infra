// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::ToSocketAddrs;
use std::sync::Mutex;

use anyhow::Result;
use flutter_rust_bridge::{RustOpaque, StreamSink};
use phnxapiclient::qs_api::ws::WsEvent;
use phnxbackend::auth_service::UserName;
use phnxserver::endpoints::qs::ws::QsWsMessage;

pub use crate::types::Conversation;
pub use crate::types::*;
use crate::{
    notifications::{Notifiable, NotificationHub},
    users::SelfUser,
};

#[path = "../dart-bridge/bridge_generated.rs"]
mod bridge_generated;

/// This is only to tell flutter_rust_bridge that it should expose the types
/// used in the parameters
pub fn _expose_conversation(conversation: Conversation) -> Conversation {
    conversation
}
pub fn _expose_notification_type(notification_type: NotificationType) -> NotificationType {
    notification_type
}

pub enum WsNotification {
    Connected,
    Disconnected,
    QueueUpdate,
}

#[derive(Clone)]
pub struct DartNotifier {
    pub stream_sink: StreamSink<NotificationType>,
}

impl Notifiable for DartNotifier {
    fn notify(&self, notification_type: NotificationType) -> bool {
        self.stream_sink.add(notification_type)
    }
}

impl From<StreamSink<NotificationType>> for DartNotifier {
    fn from(stream_sink: StreamSink<NotificationType>) -> Self {
        Self { stream_sink }
    }
}

pub struct UserBuilder {
    pub user: RustOpaque<Mutex<Option<RustUser>>>,
}

impl UserBuilder {
    pub fn new() -> UserBuilder {
        Self {
            user: RustOpaque::new(Mutex::new(None)),
        }
    }

    pub fn create_user(
        &self,
        user_name: String,
        password: String,
        address: String,
        stream_sink: StreamSink<NotificationType>,
    ) -> Result<()> {
        let user = RustUser::new(user_name, password, address, stream_sink.clone())?;
        if let Ok(mut inner_user) = self.user.try_lock() {
            let _ = inner_user.insert(user);
            // Send an initial notification to the flutter side, since this
            // function cannot be async
            stream_sink.add(NotificationType::ConversationChange(UuidBytes {
                bytes: [0; 16],
            }));
            Ok(())
        } else {
            return Err(anyhow::anyhow!("Could not acquire lock"));
        }
    }

    pub fn into_user(&self) -> Result<RustUser> {
        if let Ok(mut inner_user) = self.user.try_lock() {
            if let Some(user) = inner_user.take() {
                return Ok(user);
            } else {
                return Err(anyhow::anyhow!("User not created"));
            }
        } else {
            Err(anyhow::anyhow!("Could not acquire lock"))
        }
    }
}

pub struct RustUser {
    user: RustOpaque<Mutex<SelfUser<DartNotifier>>>,
}

impl RustUser {
    #[tokio::main(flavor = "current_thread")]
    async fn new(
        user_name: String,
        password: String,
        address: String,
        stream_sink: StreamSink<NotificationType>,
    ) -> Result<RustUser> {
        let dart_notifier = DartNotifier { stream_sink };
        let mut notification_hub = NotificationHub::<DartNotifier>::default();
        notification_hub.add_sink(dart_notifier.notifier());
        let user = SelfUser::new(
            &user_name,
            &password,
            address.to_socket_addrs().unwrap().next().unwrap(),
            notification_hub,
        )
        .await?;
        Ok(Self {
            user: RustOpaque::new(Mutex::new(user)),
        })
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn websocket(
        &self,
        timeout: u64,
        retry_interval: u64,
        stream_sink: StreamSink<WsNotification>,
    ) -> Result<()> {
        let mut user = self.user.lock().unwrap();
        let mut qs_websocket = user.websocket(timeout, retry_interval).await?;
        drop(user);

        loop {
            match qs_websocket.next().await {
                Some(event) => match event {
                    WsEvent::ConnectedEvent => {
                        stream_sink.add(WsNotification::Connected);
                    }
                    WsEvent::DisconnectedEvent => {
                        stream_sink.add(WsNotification::Disconnected);
                    }
                    WsEvent::MessageEvent(e) => match e {
                        QsWsMessage::QueueUpdate => {
                            stream_sink.add(WsNotification::QueueUpdate);
                        }
                        _ => {}
                    },
                },
                None => {
                    stream_sink.add(WsNotification::Disconnected);
                    break;
                }
            }
        }
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn create_connection(&self, user_name: String) -> Result<()> {
        let mut user = self.user.lock().unwrap();
        user.add_contact(&user_name).await?;
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn fetch_messages(&self) -> Result<()> {
        let mut user = self.user.lock().unwrap();

        let as_messages = user.as_fetch_messages().await?;
        user.process_as_messages(as_messages).await.unwrap();

        let qs_messages = user.qs_fetch_messages().await?;
        user.process_qs_messages(qs_messages).await.unwrap();

        Ok(())
    }

    pub fn get_conversations(&self) -> Vec<Conversation> {
        let user = self.user.lock().unwrap();
        user.get_conversations()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn send_message(
        &self,
        conversation_id: UuidBytes,
        message: MessageContentType,
    ) -> ConversationMessage {
        let mut user = self.user.lock().unwrap();
        user.send_message(conversation_id.as_uuid(), message)
            .await
            .unwrap()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_messages(
        &self,
        conversation_id: UuidBytes,
        last_n: usize,
    ) -> Vec<ConversationMessage> {
        let user = self.user.lock().unwrap();
        let messages = user.get_messages(conversation_id.as_uuid(), last_n);
        messages
    }

    pub fn get_contacts(&self) -> Vec<String> {
        let user = self.user.lock().unwrap();
        user.contacts()
            .into_iter()
            .map(|c| c.user_name.to_string())
            .collect()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn create_conversation(&self, name: String) -> UuidBytes {
        let mut user = self.user.lock().unwrap();
        UuidBytes::from_uuid(user.create_conversation(&name).await.unwrap())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn add_users_to_conversation(
        &self,
        conversation_id: UuidBytes,
        user_names: Vec<String>,
    ) -> Result<()> {
        let mut user = self.user.lock().unwrap();
        user.invite_users(
            conversation_id.as_uuid(),
            &user_names
                .into_iter()
                .map(UserName::from)
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn remove_users_from_conversation(
        &self,
        conversation_id: UuidBytes,
        user_names: Vec<String>,
    ) -> Result<()> {
        let mut user = self.user.lock().unwrap();
        user.remove_users(
            conversation_id.as_uuid(),
            &user_names
                .into_iter()
                .map(UserName::from)
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
        Ok(())
    }
}
