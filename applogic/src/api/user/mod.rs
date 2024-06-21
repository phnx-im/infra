// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use phnxapiclient::qs_api::ws::WsEvent;
use phnxcoreclient::{
    clients::{store::ClientRecord, CoreUser},
    UserProfile,
};
use phnxtypes::messages::client_ds::QsWsMessage;

use crate::{
    api::types::{UiNotificationType, UiUserProfile},
    app_state::state::AppState,
    notifications::{Notifiable, NotificationHub},
    StreamSink,
};

pub(crate) use phnxcoreclient::NotificationType;

pub mod connections;

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

pub struct User {
    pub(crate) user: CoreUser,
    pub(crate) app_state: AppState,
    pub(crate) notification_hub: NotificationHub<DartNotifier>,
}

impl User {
    #[tokio::main(flavor = "current_thread")]
    pub async fn new(
        user_name: String,
        password: String,
        address: String,
        path: String,
    ) -> Result<User> {
        let user = CoreUser::new(&user_name, &password, address, &path).await?;

        Ok(Self {
            user: user.clone(),
            app_state: AppState::new(user),
            notification_hub: NotificationHub::<DartNotifier>::default(),
        })
    }

    #[tokio::main(flavor = "current_thread")]
    pub async fn load_default(path: String) -> Result<User> {
        let client_record = ClientRecord::load_all_from_phnx_db(&path)?
            .pop()
            .ok_or_else(|| anyhow!("No user found."))?;
        let as_client_id = client_record.as_client_id;
        let user = CoreUser::load(as_client_id.clone(), &path)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Could not load user with client_id {}",
                    as_client_id.to_string()
                )
            })?;

        Ok(Self {
            user: user.clone(),
            app_state: AppState::new(user),
            notification_hub: NotificationHub::<DartNotifier>::default(),
        })
    }

    pub async fn notification_stream(
        &self,
        stream_sink: StreamSink<UiNotificationType>,
    ) -> Result<()> {
        self.notification_hub
            .add_sink(DartNotifier::from(stream_sink).notifier())
            .await;
        Ok(())
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
