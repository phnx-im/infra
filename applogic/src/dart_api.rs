// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use flutter_rust_bridge::{handler::DefaultHandler, support::lazy_static, RustOpaque, StreamSink};
use phnxapiclient::qs_api::ws::WsEvent;
use phnxtypes::{
    identifiers::{SafeTryInto, UserName},
    messages::client_ds::QsWsMessage,
    time::TimeStamp,
};

pub use crate::types::{UiConversation, UiConversationMessage, UiNotificationType};
use crate::{
    app_state::AppState,
    notifications::{Notifiable, NotificationHub},
    types::{ConversationIdBytes, UiContact, UiUserProfile},
};
use phnxcoreclient::{
    clients::{process::ProcessQsMessageResult, store::ClientRecord, SelfUser},
    ConversationId, ConversationMessage, MimiContent, NotificationType, UserProfile,
};

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use notify_rust::Notification;

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

pub fn delete_databases(client_db_path: String) -> Result<()> {
    phnxcoreclient::delete_databases(client_db_path.as_str())
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
    stream_sink: RustOpaque<Mutex<Option<StreamSink<UiNotificationType>>>>,
}

impl UserBuilder {
    pub fn new() -> UserBuilder {
        let _ = simple_logger::init_with_level(log::Level::Info);
        Self {
            stream_sink: RustOpaque::new(Mutex::new(None)),
        }
    }

    /// Set the stream sink that will be used to send notifications to Dart. On
    /// the Dart side, this doesn't wait for the stream sink to be set
    /// internally, but immediately returns a stream. To confirm that the stream
    /// sink is set, this function sends a first notification to the Dart side.
    pub fn get_stream(&self, stream_sink: StreamSink<UiNotificationType>) -> Result<()> {
        let mut stream_sink_option = self
            .stream_sink
            .lock()
            .map_err(|e| anyhow!("Lock error: {:?}", e))?;
        let stream_sink = stream_sink_option.insert(stream_sink);
        // Since the function will return immediately we send a first
        // notification to the Dart side so we can wait for it there.
        stream_sink.add(UiNotificationType::ConversationChange(
            ConversationIdBytes { bytes: [0; 16] },
        ));
        Ok(())
    }

    pub fn load_default(&self, path: String) -> Result<RustUser> {
        let mut stream_sink_option = self
            .stream_sink
            .lock()
            .map_err(|e| anyhow!("Lock error: {:?}", e))?;
        if let Some(stream_sink) = stream_sink_option.take() {
            RustUser::load_default(path, stream_sink)
        } else {
            return Err(anyhow::anyhow!("Please set a stream sink first."));
        }
    }

    pub fn create_user(
        &self,
        user_name: String,
        password: String,
        address: String,
        path: String,
    ) -> Result<RustUser> {
        let mut stream_sink_option = self
            .stream_sink
            .lock()
            .map_err(|e| anyhow!("Lock error: {:?}", e))?;
        if let Some(stream_sink) = stream_sink_option.take() {
            RustUser::new(user_name, password, address, path, stream_sink.clone())
        } else {
            return Err(anyhow::anyhow!("Please set a stream sink first."));
        }
    }
}

type DartNotificationHub = NotificationHub<DartNotifier>;

pub struct RustUser {
    user: RustOpaque<Arc<Mutex<SelfUser>>>,
    app_state: RustOpaque<AppState>,
    notification_hub_option: RustOpaque<Mutex<DartNotificationHub>>,
}

impl RustUser {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    fn init_desktop_os_notifications() -> Result<(), notify_rust::error::Error> {
        #[cfg(target_os = "macos")]
        {
            let res = notify_rust::set_application(&"im.phnx.prototype");
            if res.is_err() {
                log::warn!("Could not set application for desktop notifications");
            }
        }

        Ok(())
    }

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
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        Self::init_desktop_os_notifications()?;
        let user = Arc::new(Mutex::new(user));
        Ok(Self {
            user: RustOpaque::new(user.clone()),
            app_state: RustOpaque::new(AppState::new(user)),
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
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        Self::init_desktop_os_notifications()?;
        let user = Arc::new(Mutex::new(user));
        Ok(Self {
            user: RustOpaque::new(user.clone()),
            app_state: RustOpaque::new(AppState::new(user)),
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
        self.dispatch_conversation_notifications(vec![conversation_id.into()]);
        Ok(())
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn fetch_messages(&self) -> Result<()> {
        let mut user = self.user.lock().unwrap();

        // Fetch AS messages
        let as_messages = user.as_fetch_messages().await?;

        // Process each as message individually and dispatch conversation
        // notifications to the UI in case a new conversation is created.
        let mut new_connections = vec![];
        for as_message in as_messages {
            let as_message_plaintext = user.decrypt_as_queue_message(as_message)?;
            let conversation_id = user.process_as_message(as_message_plaintext).await?;
            // Let the UI know that there'a s new conversation
            self.dispatch_conversation_notifications(vec![conversation_id]);
            new_connections.push(conversation_id);
        }

        // Send a notification to the OS (desktop only), the UI deals with
        // mobile notifications
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        self.send_desktop_os_connection_notifications(&user, new_connections)?;

        // Fetch QS messages
        let qs_messages = user.qs_fetch_messages().await?;
        // Process each qs message individually and dispatch conversation message notifications
        let mut new_conversations = vec![];
        let mut new_messages = vec![];
        for qs_message in qs_messages {
            let qs_message_plaintext = user.decrypt_qs_queue_message(qs_message)?;
            match user.process_qs_message(qs_message_plaintext).await? {
                ProcessQsMessageResult::ConversationId(conversation_id) => {
                    new_conversations.push(conversation_id);
                }
                ProcessQsMessageResult::ConversationMessages(conversation_messages) => {
                    new_messages.extend(conversation_messages);
                }
            };
        }
        // Let the UI know there is new stuff
        self.dispatch_message_notifications(new_messages.clone());
        self.dispatch_conversation_notifications(new_conversations.clone());

        // Send a notification to the OS (desktop only)
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.send_desktop_os_message_notifications(&user, new_messages)?;
            self.send_desktop_os_conversation_notifications(&user, new_conversations.clone())?;
        }

        // Update user auth keys of newly created conversations.
        let mut new_messages = vec![];
        for conversation_id in new_conversations {
            let messages = user.update_user_key(conversation_id).await?;
            new_messages.extend(messages);
        }

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            self.send_desktop_os_message_notifications(&user, new_messages)?;
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
        message: String,
    ) -> Result<UiConversationMessage> {
        let mut user = self.user.lock().unwrap();
        let content = MimiContent::simple_markdown_message(user.user_name().domain(), message);
        user.send_message(conversation_id.into(), content)
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
        user.get_messages(conversation_id.into(), last_n)
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.into())
            .collect()
    }

    pub fn get_contacts(&self) -> Vec<UiContact> {
        let user = self.user.lock().unwrap();
        user.contacts()
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.into())
            .collect()
    }

    pub fn get_contact(&self, user_name: String) -> Option<UiContact> {
        let user = self.user.lock().unwrap();
        let user_name = <String as SafeTryInto<UserName>>::try_into(user_name).unwrap();
        user.contact(&user_name).map(|c| c.into())
    }

    /// Get the user profile of the user with the given [`UserName`].
    pub fn user_profile(&self, user_name: String) -> Result<Option<UiUserProfile>> {
        let user = self.user.lock().unwrap();
        let user_name = SafeTryInto::try_into(user_name)?;
        let user_profile = user
            .user_profile(&user_name)?
            .map(|up| UiUserProfile::from(up).into());
        Ok(user_profile)
    }

    /// Get the own user profile.
    pub fn own_user_profile(&self) -> Result<UiUserProfile> {
        let user = self.user.lock().unwrap();
        let user_profile = user
            .own_user_profile()
            .map(|up| UiUserProfile::from(up).into())?;
        Ok(user_profile)
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
                    .map(|s| <String as SafeTryInto<UserName>>::try_into(s))
                    .collect::<Result<Vec<UserName>, _>>()?,
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
                    .map(|s| <String as SafeTryInto<UserName>>::try_into(s))
                    .collect::<Result<Vec<UserName>, _>>()?,
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
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.to_string())
            .collect())
    }

    // TODO: This does not yet send the new user profile to other clients
    #[tokio::main(flavor = "current_thread")]
    pub async fn set_user_profile(
        &self,
        display_name: String,
        profile_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let user = self.user.lock().unwrap();
        let ui_user_profile = UiUserProfile {
            display_name: Some(display_name),
            user_name: self.user_name(),
            profile_picture_option,
        };
        let user_profile = UserProfile::try_from(ui_user_profile)?;
        user.set_own_user_profile(user_profile)?;
        Ok(())
    }

    /// This function is called from the flutter side to mark messages as read.
    ///
    /// The function is debounced and can be called multiple times in quick
    /// succession.
    pub fn mark_messages_as_read_debounced(
        &self,
        conversation_id: ConversationIdBytes,
        timestamp: u64,
    ) -> Result<()> {
        let timestamp = TimeStamp::try_from(timestamp)?;
        self.app_state
            .mark_messages_read_debounced(conversation_id.into(), timestamp)
    }

    /// This function is called from the flutter side to flush the debouncer
    /// state, immediately terminating the debouncer and marking all pending
    /// messages as read.
    pub fn flush_debouncer_state(&self) -> Result<()> {
        self.app_state.flush_debouncer_state()
    }

    /// Get a list of contacts to be added to the conversation with the given
    /// [`ConversationId`].
    pub fn member_candidates(
        &self,
        conversation_id: ConversationIdBytes,
    ) -> Result<Vec<UiContact>> {
        let user = self.user.lock().unwrap();
        let group_members = user
            .group_members(conversation_id.into())
            .ok_or(anyhow!("Conversation not found"))?;
        let add_candidates = user
            .contacts()?
            .into_iter()
            .filter_map(|c| {
                if !group_members.contains(&c.user_name) {
                    Some(c.into())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(add_candidates)
    }

    /// Dispatch a notification to the flutter side if and only if a
    /// notification hub is set.
    fn dispatch_conversation_notifications(
        &self,
        conversation_ids: impl IntoIterator<Item = ConversationId>,
    ) {
        let mut notification_hub = self.notification_hub_option.lock().unwrap();
        conversation_ids.into_iter().for_each(|conversation_id| {
            notification_hub.dispatch_conversation_notification(conversation_id.into())
        });
    }

    /// Dispatch conversation message notifications to the flutter side if and
    /// only if a notification hub is set.
    fn dispatch_message_notifications(
        &self,
        conversation_messages: impl IntoIterator<Item = ConversationMessage>,
    ) {
        let mut notification_hub = self.notification_hub_option.lock().unwrap();
        conversation_messages
            .into_iter()
            .for_each(|conversation_message| {
                notification_hub.dispatch_message_notification(conversation_message.into())
            });
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    fn send_desktop_os_message_notifications(
        &self,
        user: &SelfUser,
        conversation_messages: Vec<ConversationMessage>,
    ) -> Result<()> {
        let (summary, body) = match &conversation_messages[..] {
            [] => return Ok(()),
            [conversation_message] => {
                let conversation = user
                    .conversation(conversation_message.conversation_id())
                    .ok_or(anyhow!("Conversation not found"))?;
                let summary = match conversation.conversation_type() {
                    phnxcoreclient::ConversationType::UnconfirmedConnection(username)
                    | phnxcoreclient::ConversationType::Connection(username) => {
                        username.to_string()
                    }
                    phnxcoreclient::ConversationType::Group => {
                        conversation.attributes().title().to_string()
                    }
                };
                let body = conversation_message
                    .message()
                    .string_representation(conversation.conversation_type());
                (summary, body)
            }
            _ => (
                "New messages".to_owned(),
                "You have received new messages.".to_owned(),
            ),
        };

        Notification::new()
            .summary(summary.as_str())
            .body(body.as_str())
            .show()?;

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    fn send_desktop_os_conversation_notifications(
        &self,
        user: &SelfUser,
        conversations: Vec<ConversationId>,
    ) -> Result<()> {
        let (summary, body) = match conversations[..] {
            [] => return Ok(()),
            [conversation] => {
                let conversation_title = user
                    .conversation(conversation)
                    .ok_or(anyhow!("Conversation not found"))?
                    .attributes()
                    .title()
                    .to_string();
                let summary = "New conversation";
                let body = format!("You have been added to {}", conversation_title);
                (summary, body)
            }
            _ => {
                let summary = "New conversations";
                let body = "You have been added to new conversations.".to_owned();
                (summary, body)
            }
        };

        Notification::new()
            .summary(summary)
            .body(body.as_str())
            .show()?;

        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    fn send_desktop_os_connection_notifications(
        &self,
        user: &SelfUser,
        connection_conversations: Vec<ConversationId>,
    ) -> Result<()> {
        let (summary, body) = match connection_conversations[..] {
            [] => return Ok(()),
            [conversation] => {
                let conversation = user
                    .conversation(conversation)
                    .ok_or(anyhow!("Conversation not found"))?;
                let contact_name = match conversation.conversation_type() {
                    phnxcoreclient::ConversationType::UnconfirmedConnection(username)
                    | phnxcoreclient::ConversationType::Connection(username) => {
                        username.to_string()
                    }
                    phnxcoreclient::ConversationType::Group => {
                        return Err(anyhow!(
                            "Conversation is a regular group, not a connection."
                        ))
                    }
                };
                let summary = "New connection";
                let body = format!("{} has created a new connection with you.", contact_name);
                (summary, body)
            }
            _ => {
                let summary = "New connections";
                let body = "Multiple new connections have been created.".to_owned();
                (summary, body)
            }
        };

        Notification::new()
            .summary(summary)
            .body(body.as_str())
            .show()?;

        Ok(())
    }
}
