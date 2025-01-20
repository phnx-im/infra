// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{anyhow, Result};
use phnxapiclient::qs_api::ws::WsEvent;
use phnxcoreclient::{
    clients::{store::ClientRecord, CoreUser},
    Asset, UserProfile,
};
use phnxtypes::{
    identifiers::QualifiedUserName,
    messages::{client_ds::QsWsMessage, push_token::PushTokenOperator},
};
use tracing::error;

use crate::{
    api::types::UiNotificationType,
    app_state::state::AppState,
    notifier::{Notifiable, NotificationHub},
    StreamSink,
};

pub(crate) use phnxcoreclient::NotificationType;
pub(crate) use phnxtypes::messages::push_token::PushToken;

pub mod connections;
pub mod user_cubit;

pub enum PlatformPushToken {
    Apple(String),
    Google(String),
}

impl From<PlatformPushToken> for PushToken {
    fn from(platform_push_token: PlatformPushToken) -> Self {
        match platform_push_token {
            PlatformPushToken::Apple(token) => PushToken::new(PushTokenOperator::Apple, token),
            PlatformPushToken::Google(token) => PushToken::new(PushTokenOperator::Google, token),
        }
    }
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
    pub(crate) fn with_empty_state(core_user: CoreUser) -> Self {
        Self {
            user: core_user.clone(),
            app_state: AppState::new(core_user),
            notification_hub: Default::default(),
        }
    }

    pub async fn new(
        user_name: String,
        password: String,
        address: String,
        path: String,
        push_token: Option<PlatformPushToken>,
        display_name: Option<String>,
        profile_picture: Option<Vec<u8>>,
    ) -> Result<User> {
        let user_name: QualifiedUserName = user_name.parse()?;
        let user_profile = UserProfile::new(
            user_name.clone(),
            display_name.map(TryFrom::try_from).transpose()?,
            profile_picture.map(Asset::Value),
        );

        let user = CoreUser::new(
            user_name.clone(),
            &password,
            address,
            &path,
            push_token.map(|p| p.into()),
        )
        .await?;

        if let Err(error) = CoreUser::set_own_user_profile(&user, user_profile).await {
            error!(%error, "Could not set own user profile");
        }

        Ok(Self {
            user: user.clone(),
            app_state: AppState::new(user),
            notification_hub: NotificationHub::<DartNotifier>::default(),
        })
    }

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

    /// Update the push token.
    pub async fn update_push_token(&self, push_token: Option<PlatformPushToken>) -> Result<()> {
        self.user
            .update_push_token(push_token.map(|p| p.into()))
            .await?;
        Ok(())
    }
}
