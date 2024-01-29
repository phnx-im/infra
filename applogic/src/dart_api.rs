// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Mutex;

use anyhow::Result;
use flutter_rust_bridge::{handler::DefaultHandler, support::lazy_static, RustOpaque, StreamSink};
use phnxapiclient::qs_api::ws::WsEvent;
use phnxtypes::{identifiers::UserName, messages::client_ds::QsWsMessage};

pub use crate::types::{
    UiConversation, UiConversationMessage, UiMessageContentType, UiNotificationType,
};
use crate::{
    notifications::{Notifiable, NotificationHub},
    types::{ConversationIdBytes, UiContact},
};
use phnxcoreclient::{
    users::{store::ClientRecord, SelfUser},
    ConversationMessage, NotificationType,
};

lazy_static! {
    static ref FLUTTER_RUST_BRIDGE_HANDLER: DefaultHandler = DefaultHandler::default();
}

#[path = "../dart-bridge/bridge_generated.rs"]
mod bridge_generated;

/// This is only to tell flutter_rust_bridge that it should expose the types
/// used in the parameters
pub fn _expose_conversation(conversation: UiConversation) -> UiConversation {
    conversation
}
pub fn _expose_notification_type(notification_type: UiNotificationType) -> UiNotificationType {
    notification_type
}

pub enum WsNotification {
    Connected,
    Disconnected,
    QueueUpdate,
}

#[derive(Clone)]
pub struct DartNotifier {
    pub stream_sink: StreamSink<UiNotificationType>,
}

impl Notifiable for DartNotifier {
    fn notify(&self, notification_type: NotificationType) -> bool {
        let ui_notification_type = UiNotificationType::from(notification_type);
        self.stream_sink.add(ui_notification_type)
    }
}

impl From<StreamSink<UiNotificationType>> for DartNotifier {
    fn from(stream_sink: StreamSink<UiNotificationType>) -> Self {
        Self { stream_sink }
    }
}

pub struct UserBuilder {
    pub user: RustOpaque<Mutex<Option<RustUser>>>,
}

impl UserBuilder {
    pub fn new() -> UserBuilder {
        let _ = simple_logger::init_with_level(log::Level::Info);
        Self {
            user: RustOpaque::new(Mutex::new(None)),
        }
    }

    pub fn load_default(
        &self,
        path: String,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<()> {
        let user = RustUser::load_default(path, stream_sink.clone())?;
        if let Ok(mut inner_user) = self.user.try_lock() {
            let _ = inner_user.insert(user);
            // Send an initial notification to the flutter side, since this
            // function cannot be async
            stream_sink.add(UiNotificationType::ConversationChange(
                ConversationIdBytes { bytes: [0; 16] },
            ));
            Ok(())
        } else {
            return Err(anyhow::anyhow!("Could not acquire lock"));
        }
    }

    pub fn create_user(
        &self,
        user_name: String,
        password: String,
        address: String,
        path: String,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<()> {
        let user = RustUser::new(user_name, password, address, path, stream_sink.clone())?;
        if let Ok(mut inner_user) = self.user.try_lock() {
            let _ = inner_user.insert(user);
            // Send an initial notification to the flutter side, since this
            // function cannot be async
            stream_sink.add(UiNotificationType::ConversationChange(
                ConversationIdBytes { bytes: [0; 16] },
            ));
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

type DartNotificationHub = NotificationHub<DartNotifier>;

pub struct RustUser {
    user: RustOpaque<Mutex<SelfUser>>,
    notification_hub_option: RustOpaque<Mutex<DartNotificationHub>>,
}

impl RustUser {
    #[tokio::main(flavor = "current_thread")]
    async fn new(
        user_name: String,
        password: String,
        address: String,
        path: String,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<RustUser> {
        let dart_notifier = DartNotifier { stream_sink };
        let mut notification_hub = NotificationHub::<DartNotifier>::default();
        notification_hub.add_sink(dart_notifier.notifier());
        let user = SelfUser::new(&user_name, &password, address, &path).await?;
        Ok(Self {
            user: RustOpaque::new(Mutex::new(user)),
            notification_hub_option: RustOpaque::new(Mutex::new(notification_hub)),
        })
    }

    #[tokio::main(flavor = "current_thread")]
    async fn load_default(
        path: String,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<RustUser> {
        let client_record = ClientRecord::load_all(&path)?.pop().ok_or_else(|| {
            anyhow::anyhow!("No user found. Please create a user first using createUser")
        })?;
        let dart_notifier = DartNotifier { stream_sink };
        let mut notification_hub = NotificationHub::<DartNotifier>::default();
        notification_hub.add_sink(dart_notifier.notifier());
        let as_client_id = client_record.as_client_id;
        let user = SelfUser::load(as_client_id.clone(), &path)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not load user with client_id {}",
                    as_client_id.to_string()
                )
            })?;
        Ok(Self {
            user: RustOpaque::new(Mutex::new(user)),
            notification_hub_option: RustOpaque::new(Mutex::new(notification_hub)),
        })
    }

    pub fn user_name(&self) -> String {
        let user = self.user.lock().unwrap();
        user.user_name().to_string()
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
        let conversation_id = user.add_contact(&user_name).await?;
        self.dispatch_conversation_notification(conversation_id.into());
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn fetch_messages(&self) -> Result<()> {
        let mut user = self.user.lock().unwrap();

        // Fetch AS messages
        let as_messages = user.as_fetch_messages().await?;
        // Process each as message individually and dispatch conversation
        // notifications to the UI in case a new conversation is created.
        for as_message in as_messages {
            let as_message_plaintext = user.decrypt_as_queue_message(as_message)?;
            let conversation_id = user.process_as_message(as_message_plaintext).await?;
            self.dispatch_conversation_notification(conversation_id.into());
        }

        // Fetch QS messages
        let qs_messages = user.qs_fetch_messages().await?;
        // Process each qs message individually and dispatch conversation message notifications
        for qs_message in qs_messages {
            let qs_message_plaintext = user.decrypt_qs_queue_message(qs_message)?;
            let conversation_messages = user.process_qs_message(qs_message_plaintext).await?;
            self.dispatch_message_notifications(conversation_messages);
        }

        Ok(())
    }

    pub fn get_conversations(&self) -> Vec<UiConversation> {
        let user = self.user.lock().unwrap();
        user.conversations()
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn send_message(
        &self,
        conversation_id: ConversationIdBytes,
        message: UiMessageContentType,
    ) -> Result<UiConversationMessage> {
        let mut user = self.user.lock().unwrap();
        user.send_message(conversation_id.into(), message.into())
            .await
            .map(|m| m.into())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn get_messages(
        &self,
        conversation_id: ConversationIdBytes,
        last_n: usize,
    ) -> Vec<UiConversationMessage> {
        let user = self.user.lock().unwrap();
        let messages = user
            .get_messages(conversation_id.into(), last_n)
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.into())
            .collect();
        messages
    }

    pub fn get_contacts(&self) -> Vec<UiContact> {
        let user = self.user.lock().unwrap();
        user.contacts()
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn create_conversation(&self, name: String) -> Result<ConversationIdBytes> {
        let mut user = self.user.lock().unwrap();
        Ok(ConversationIdBytes::from(
            user.create_conversation(&name, None).await?,
        ))
    }

    pub fn set_conversation_picture(
        &self,
        conversation_id: ConversationIdBytes,
        conversation_picture: Option<Vec<u8>>,
    ) -> Result<()> {
        let user = self.user.lock().unwrap();
        user.set_conversation_picture(conversation_id.into(), conversation_picture)?;
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn add_users_to_conversation(
        &self,
        conversation_id: ConversationIdBytes,
        user_names: Vec<String>,
    ) -> Result<()> {
        let mut user = self.user.lock().unwrap();
        let conversation_messages = user
            .invite_users(
                conversation_id.into(),
                &user_names
                    .into_iter()
                    .map(UserName::from)
                    .collect::<Vec<_>>(),
            )
            .await?;
        self.dispatch_message_notifications(conversation_messages);
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn remove_users_from_conversation(
        &self,
        conversation_id: ConversationIdBytes,
        user_names: Vec<String>,
    ) -> Result<()> {
        let mut user = self.user.lock().unwrap();
        let conversation_messages = user
            .remove_users(
                conversation_id.into(),
                &user_names
                    .into_iter()
                    .map(UserName::from)
                    .collect::<Vec<_>>(),
            )
            .await?;
        self.dispatch_message_notifications(conversation_messages);
        Ok(())
    }

    pub fn members_of_conversation(
        &self,
        conversation_id: ConversationIdBytes,
    ) -> Result<Vec<String>> {
        let user = self.user.lock().unwrap();
        Ok(user
            .group_members(conversation_id.into())
            .unwrap_or(Vec::new())
            .into_iter()
            .map(|c| c.to_string())
            .collect())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn set_user_profile(
        &self,
        display_name: String,
        profile_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let user = self.user.lock().unwrap();
        user.store_user_profile(display_name, profile_picture_option)
            .await
    }

    /// Dispatch a notification to the flutter side if and only if a
    /// notification hub is set.
    fn dispatch_conversation_notification(&self, conversation_id: ConversationIdBytes) {
        let mut notification_hub = self.notification_hub_option.lock().unwrap();
        notification_hub.dispatch_conversation_notification(conversation_id.into())
    }

    /// Dispatch conversation message notifications to the flutter side if and
    /// only if a notification hub is set.
    fn dispatch_message_notifications(&self, conversation_messages: Vec<ConversationMessage>) {
        let mut notification_hub = self.notification_hub_option.lock().unwrap();
        conversation_messages
            .into_iter()
            .for_each(|conversation_message| {
                notification_hub.dispatch_message_notification(conversation_message.into())
            });
    }
}
