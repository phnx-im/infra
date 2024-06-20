// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use phnxapiclient::qs_api::ws::WsEvent;
use phnxcoreclient::{
    clients::{store::ClientRecord, CoreUser},
    NotificationType, UserProfile,
};
use phnxtypes::messages::client_ds::QsWsMessage;
use tokio::sync::Mutex;

use crate::{
    api::{
        types::{ConversationIdBytes, UiNotificationType, UiUserProfile},
        utils::rust_set_up,
    },
    app_state::state::AppState,
    notifications::{Notifiable, NotificationHub},
    StreamSink,
};

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
        self.stream_sink.add(ui_notification_type).is_ok()
    }
}

impl From<StreamSink<UiNotificationType>> for DartNotifier {
    fn from(stream_sink: StreamSink<UiNotificationType>) -> Self {
        Self { stream_sink }
    }
}

pub struct UserBuilder {
    stream_sink: Mutex<Option<StreamSink<UiNotificationType>>>,
}

type DartNotificationHub = NotificationHub<DartNotifier>;

pub struct User {
    pub(crate) user: CoreUser,
    pub(crate) app_state: AppState,
    pub(crate) notification_hub_option: Mutex<DartNotificationHub>,
}

impl UserBuilder {
    pub fn new() -> UserBuilder {
        rust_set_up();
        Self {
            stream_sink: Mutex::new(None),
        }
    }

    /// Set the stream sink that will be used to send notifications to Dart. On
    /// the Dart side, this doesn't wait for the stream sink to be set
    /// internally, but immediately returns a stream. To confirm that the stream
    /// sink is set, this function sends a first notification to the Dart side.
    pub async fn get_stream(&self, stream_sink: StreamSink<UiNotificationType>) -> Result<()> {
        let mut stream_sink_option = self.stream_sink.lock().await;
        let stream_sink = stream_sink_option.insert(stream_sink);
        // Since the function will return immediately we send a first
        // notification to the Dart side so we can wait for it there.
        stream_sink
            .add(UiNotificationType::ConversationChange(
                ConversationIdBytes { bytes: [0; 16] },
            ))
            .map_err(|e| anyhow!("Error sending notification: {:?}", e))
    }

    pub async fn load_default(&self, path: String) -> Result<User> {
        let mut stream_sink_option = self.stream_sink.lock().await;
        let Some(stream_sink) = stream_sink_option.take() else {
            return Err(anyhow::anyhow!("Please set a stream sink first."));
        };
        drop(stream_sink_option);

        User::load_default(path, stream_sink).await
    }

    pub async fn create_user(
        &self,
        user_name: String,
        password: String,
        address: String,
        path: String,
    ) -> Result<User> {
        let mut stream_sink_option = self.stream_sink.lock().await;
        let Some(stream_sink) = stream_sink_option.take() else {
            return Err(anyhow::anyhow!("Please set a stream sink first."));
        };
        drop(stream_sink_option);

        User::new(user_name, password, address, path, stream_sink.clone()).await
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl User {
    async fn new(
        user_name: String,
        password: String,
        address: String,
        path: String,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<User> {
        let dart_notifier = DartNotifier { stream_sink };
        let mut notification_hub = NotificationHub::<DartNotifier>::default();
        notification_hub.add_sink(dart_notifier.notifier());
        let user = CoreUser::new(&user_name, &password, address, &path).await?;
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        Self::init_desktop_os_notifications()?;
        Ok(Self {
            user: user.clone(),
            app_state: AppState::new(user),
            notification_hub_option: Mutex::new(notification_hub),
        })
    }

    async fn load_default(
        path: String,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<User> {
        let client_record = ClientRecord::load_all_from_phnx_db(&path)?
            .pop()
            .ok_or_else(|| {
                anyhow::anyhow!("No user found. Please create a user first using createUser")
            })?;
        let dart_notifier = DartNotifier { stream_sink };
        let mut notification_hub = NotificationHub::<DartNotifier>::default();
        notification_hub.add_sink(dart_notifier.notifier());
        let as_client_id = client_record.as_client_id;
        let user = CoreUser::load(as_client_id.clone(), &path)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not load user with client_id {}",
                    as_client_id.to_string()
                )
            })?;
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        Self::init_desktop_os_notifications()?;
        Ok(Self {
            user: user.clone(),
            app_state: AppState::new(user),
            notification_hub_option: Mutex::new(notification_hub),
        })
    }

    pub async fn user_name(&self) -> String {
        self.user.user_name().to_string()
    }

    pub async fn websocket(
        &self,
        timeout: u32,
        retry_interval: u32,
        stream_sink: StreamSink<WsNotification>,
    ) -> Result<()> {
        let mut qs_websocket = self
            .user
            .websocket(timeout as u64, retry_interval as u64)
            .await?;

        loop {
            match qs_websocket.next().await {
                Some(event) => match event {
                    WsEvent::ConnectedEvent => {
                        stream_sink
                            .add(WsNotification::Connected)
                            .map_err(|e| anyhow!(e))?;
                    }
                    WsEvent::DisconnectedEvent => {
                        stream_sink
                            .add(WsNotification::Disconnected)
                            .map_err(|e| anyhow!(e))?;
                    }
                    WsEvent::MessageEvent(QsWsMessage::QueueUpdate) => {
                        stream_sink
                            .add(WsNotification::QueueUpdate)
                            .map_err(|e| anyhow!(e))?;
                    }
                    _ => {}
                },
                None => {
                    stream_sink
                        .add(WsNotification::Disconnected)
                        .map_err(|e| anyhow!(e))?;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Get the own user profile.
    pub async fn own_user_profile(&self) -> Result<UiUserProfile> {
        let user_profile = self
            .user
            .own_user_profile()
            .await
            .map(UiUserProfile::from)?;
        Ok(user_profile)
    }

    // TODO: This does not yet send the new user profile to other clients
    pub async fn set_user_profile(
        &self,
        display_name: String,
        profile_picture_option: Option<Vec<u8>>,
    ) -> Result<()> {
        let ui_user_profile = UiUserProfile {
            display_name: Some(display_name),
            user_name: self.user.user_name().to_string(),
            profile_picture_option,
        };
        let user_profile = UserProfile::try_from(ui_user_profile)?;
        self.user.set_own_user_profile(user_profile).await?;
        Ok(())
    }
}
