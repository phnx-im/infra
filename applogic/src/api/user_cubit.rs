// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Logged-in user feature

use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use flutter_rust_bridge::frb;
use phnxcoreclient::{
    Asset, UserProfile,
    clients::{QueueEvent, queue_event},
};
use phnxcoreclient::{ConversationId, clients::CoreUser, store::Store};
use phnxtypes::identifiers::QualifiedUserName;
use tokio::sync::{RwLock, watch};
use tokio_stream::StreamExt;
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{debug, error, info, warn};

use crate::{
    StreamSink,
    api::navigation_cubit::HomeNavigationState,
    notifications::NotificationService,
    util::{FibonacciBackoff, spawn_from_sync},
};
use crate::{
    api::types::{UiContact, UiUserProfile},
    messages::FetchedMessages,
};

use super::{
    navigation_cubit::{NavigationCubitBase, NavigationState},
    types::ImageData,
    user::User,
};

/// State of the [`UserCubit`] which is the logged in user
///
/// Opaque, cheaply clonable, copy-on-write type
///
/// Note: This has a prefix `Ui` to avoid conflicts with the `User`.
//
// TODO: Currently, frb does not support exposing eq and hash to Dart. When it is possible, we
// should do it, to minimize the amount of UI rebuilds in Flutter.
//
// See:
// * <https://github.com/phnx-im/infra/issues/247>
// * <https://github.com/fzyzcjy/flutter_rust_bridge/issues/2238>
#[frb(opaque)]
#[derive(Debug, Clone)]
pub struct UiUser {
    inner: Arc<UiUserInner>,
}

#[derive(Debug)]
struct UiUserInner {
    user_name: QualifiedUserName,
    profile: UserProfile,
}

impl UiUser {
    fn new(user_name: QualifiedUserName, profile: UserProfile) -> Self {
        let inner = Arc::new(UiUserInner { user_name, profile });
        Self { inner }
    }

    /// Loads the user profile in the background
    fn spawn_load(this: Arc<RwLock<Self>>, core_user: CoreUser) {
        spawn_from_sync(async move {
            match core_user.own_user_profile().await {
                Ok(profile) => {
                    let mut state = this.write().await;
                    *state = UiUser::new(state.inner.user_name.clone(), profile);
                }
                Err(error) => {
                    error!(%error, "Could not load own user profile");
                }
            }
        });
    }

    #[frb(getter, sync)]
    pub fn user_name(&self) -> String {
        self.inner.user_name.to_string()
    }

    #[frb(getter, sync)]
    pub fn display_name(&self) -> String {
        self.inner.profile.display_name.clone().to_string()
    }

    #[frb(getter, sync)]
    pub fn profile_picture(&self) -> Option<ImageData> {
        Some(ImageData::from_asset(
            self.inner.profile.profile_picture.clone()?,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppState {
    Background,
    Foreground,
}

/// Provides access to the logged in user and their profile.
///
/// Also listens to queue service messages and fetches updates from the server. The lifetime of the
/// listening stream is tied to the lifetime of the cubit.
///
/// This cubit should not be created more than once, because the logged in user exists in the
/// system only once.
///
/// Allows other cubits to listen to the messages fetched from the server. In this regard, it is
/// special because it is a constuction entry point of other cubits.
#[frb(opaque)]
pub struct UserCubitBase {
    state: Arc<RwLock<UiUser>>,
    sinks: Option<Vec<StreamSink<UiUser>>>,
    pub(crate) core_user: CoreUser,
    #[cfg_attr(
        any(target_os = "linux", target_os = "macos", target_os = "windows"),
        expect(dead_code)
    )]
    app_state_tx: watch::Sender<AppState>,
    _background_tasks_cancel: DropGuard,
}

const POLLING_INTERVAL: Duration = Duration::from_secs(10);

impl UserCubitBase {
    #[frb(sync)]
    pub fn new(user: &User, navigation: &NavigationCubitBase) -> Self {
        let core_user = user.user.clone();
        let state = Arc::new(RwLock::new(UiUser::new(
            core_user.user_name().clone(),
            UserProfile::from_user_name(core_user.user_name()),
        )));

        UiUser::spawn_load(state.clone(), core_user.clone());

        let navigation_state = navigation.subscribe();
        let notification_service = navigation.notification_service.clone();

        let (app_state_tx, app_state) = watch::channel(AppState::Foreground);

        let cancel = CancellationToken::new();
        spawn_listen(
            core_user.clone(),
            navigation_state.clone(),
            app_state.clone(),
            notification_service.clone(),
            cancel.clone(),
        );
        spawn_polling(
            core_user.clone(),
            navigation_state,
            app_state,
            notification_service.clone(),
            cancel.clone(),
        );

        Self {
            state,
            sinks: Some(Default::default()),
            core_user,
            app_state_tx,
            _background_tasks_cancel: cancel.drop_guard(),
        }
    }

    async fn emit(&mut self, state: UiUser) {
        *self.state.write().await = state.clone();
        if let Some(sinks) = &mut self.sinks {
            sinks.retain(|sink| sink.add(state.clone()).is_ok());
        }
    }

    // Cubit inteface

    pub fn close(&mut self) {
        self.sinks = None;
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.sinks.is_none()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> UiUser {
        self.state.blocking_read().clone()
    }

    pub fn stream(&mut self, sink: StreamSink<UiUser>) {
        if let Some(sinks) = &mut self.sinks {
            sinks.push(sink);
        }
    }

    // Cubit methods

    /// Set the display name and/or profile picture of the user.
    pub async fn set_profile(
        &mut self,
        display_name: Option<String>,
        profile_picture: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let display_name = display_name.map(|s| s.parse()).transpose()?;
        let profile_picture = profile_picture.map(Asset::Value);
        let user = {
            let mut state = self.state.write().await;
            let user_profile = &state.inner.profile;
            let mut user_profile = user_profile.clone();
            if let Some(value) = display_name {
                user_profile.display_name = value;
            }
            if let Some(value) = profile_picture {
                user_profile.profile_picture = Some(value);
            }
            self.core_user
                .set_own_user_profile(user_profile.clone())
                .await?;
            let user = UiUser::new(state.inner.user_name.clone(), user_profile.clone());
            *state = user.clone();
            user
        };
        self.emit(user).await;
        Ok(())
    }

    /// Get the user profile of the user with the given [`QualifiedUserName`].
    #[frb(positional)]
    pub async fn user_profile(&self, user_name: String) -> anyhow::Result<Option<UiUserProfile>> {
        let user_name = user_name.parse()?;
        let user_profile = self
            .core_user
            .user_profile(&user_name)
            .await?
            .map(|profile| UiUserProfile::from_profile(&profile));
        Ok(user_profile)
    }

    #[frb(positional)]
    pub async fn add_user_to_conversation(
        &self,
        conversation_id: ConversationId,
        user_name: String,
    ) -> anyhow::Result<()> {
        let user_name: QualifiedUserName = user_name.parse()?;
        self.core_user
            .invite_users(conversation_id, &[user_name])
            .await?;
        Ok(())
    }

    #[frb(positional)]
    pub async fn remove_user_from_conversation(
        &self,
        conversation_id: ConversationId,
        user_name: String,
    ) -> anyhow::Result<()> {
        let user_name: QualifiedUserName = user_name.parse()?;
        self.core_user
            .remove_users(conversation_id, &[user_name])
            .await?;
        Ok(())
    }

    #[frb(getter)]
    pub async fn contacts(&self) -> anyhow::Result<Vec<UiContact>> {
        let contacts = self.core_user.contacts().await?;
        Ok(contacts.into_iter().map(From::from).collect())
    }

    pub fn set_app_state(&self, app_state: AppState) {
        info!(?app_state, "app state changed");
        // Note: on Desktop, we consider the app to be always in foreground
        #[cfg(any(target_os = "android", target_os = "ios"))]
        let _no_receivers = self.app_state_tx.send(app_state);
    }
}

fn spawn_listen(
    core_user: CoreUser,
    navigation_state: watch::Receiver<NavigationState>,
    mut app_state: watch::Receiver<AppState>,
    notification_service: NotificationService,
    cancel: CancellationToken,
) {
    spawn_from_sync(async move {
        let mut backoff = FibonacciBackoff::new();
        loop {
            let res = run_listen(
                &core_user,
                &navigation_state,
                &mut app_state,
                &notification_service,
                &cancel,
                &mut backoff,
            )
            .await;

            // stop handler on cancellation
            if let Ok(true) = res {
                return;
            }

            // wait for app to be foreground
            let _ = app_state
                .wait_for(|app_state| matches!(app_state, AppState::Foreground))
                .await;

            // if listen failed, retry with backoff
            if let Err(error) = res {
                let timeout = backoff.next_backoff();
                info!(%error, retry_in =? timeout, "listen failed");

                tokio::time::sleep(timeout).await;
            }
        }
    });
}

/// Returns `true` if `cancel` was cancelled.
///
/// Otherwise, returns `false` or an error.
async fn run_listen(
    core_user: &CoreUser,
    navigation_state: &watch::Receiver<NavigationState>,
    app_state: &mut watch::Receiver<AppState>,
    notification_service: &NotificationService,
    cancel: &CancellationToken,
    backoff: &mut FibonacciBackoff,
) -> anyhow::Result<bool> {
    let mut queue_stream = core_user.listen_queue().await?;
    info!("listening to the queue");

    loop {
        let in_background =
            app_state.wait_for(|app_state| matches!(app_state, AppState::Background));

        let event = tokio::select! {
            event = queue_stream.next() => event,
            _ = in_background => return Ok(false),
            _ = cancel.cancelled() => return Ok(true),
        };

        match event {
            Some(QueueEvent { event: Some(event) }) => {
                handle_queue_event(event, core_user, navigation_state, notification_service).await
            }
            Some(QueueEvent { event: None }) => {
                error!("missing `event` field in queue event");
            }
            None => bail!("disconnected from the queue"),
        }
        backoff.reset(); // reset backoff after a successful message
    }
}

fn spawn_polling(
    core_user: CoreUser,
    navigation_state: watch::Receiver<NavigationState>,
    mut app_state: watch::Receiver<AppState>,
    notification_service: NotificationService,
    cancel: CancellationToken,
) {
    let user = User::from_core_user(core_user);
    spawn_from_sync(async move {
        let mut backoff = FibonacciBackoff::new();
        loop {
            let _ = app_state
                .wait_for(|app_state| matches!(app_state, AppState::Foreground))
                .await;

            let res = tokio::select! {
                _ = cancel.cancelled() => break,
                res = user.fetch_all_messages() => res,
            };
            let mut timeout = POLLING_INTERVAL;
            match res {
                Ok(fetched_messages) => {
                    process_fetched_messages(
                        &navigation_state,
                        &notification_service,
                        fetched_messages,
                    )
                    .await;
                    backoff.reset();
                }
                Err(error) => {
                    timeout = backoff.next_backoff().max(timeout);
                    error!(retry_in =? timeout, %error, "Failed to fetch messages");
                }
            }
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(POLLING_INTERVAL) => {},
            }
        }
    });
}

async fn handle_queue_event(
    event: queue_event::Event,
    core_user: &CoreUser,
    navigation_state: &watch::Receiver<NavigationState>,
    notification_service: &NotificationService,
) {
    debug!(?event, "handling listen event");
    match event {
        queue_event::Event::Payload(_) => {
            // currently, we don't handle payload events
            warn!("ignoring listen event")
        }
        queue_event::Event::Update(_) => {
            let core_user = core_user.clone();
            let user = User::from_core_user(core_user);
            match user.fetch_all_messages().await {
                Ok(fetched_messages) => {
                    process_fetched_messages(
                        navigation_state,
                        notification_service,
                        fetched_messages,
                    )
                    .await;
                }
                Err(error) => {
                    error!(%error, "failed to fetch messages on queue update");
                }
            }
        }
    }
}

/// Places in the app where notifications in foreground are handled differently.
///
/// Dervived from the [`NavigationState`].
#[derive(Debug)]
enum NotificationContext {
    Intro,
    Conversation(ConversationId),
    ConversationList,
    Other,
}

async fn process_fetched_messages(
    navigation_state: &watch::Receiver<NavigationState>,
    notification_service: &NotificationService,
    mut fetched_messages: FetchedMessages,
) {
    let notification_context = match &*navigation_state.borrow() {
        NavigationState::Intro { .. } => NotificationContext::Intro,
        NavigationState::Home {
            home:
                HomeNavigationState {
                    conversation_id: Some(conversation_id),
                    ..
                },
        } => NotificationContext::Conversation(*conversation_id),
        NavigationState::Home {
            home:
                HomeNavigationState {
                    conversation_id: None,
                    developer_settings_screen,
                    user_settings_open,
                    ..
                },
        } => {
            const IS_DESKTOP: bool = cfg!(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux"
            ));
            if !IS_DESKTOP && developer_settings_screen.is_none() && !*user_settings_open {
                NotificationContext::ConversationList
            } else {
                NotificationContext::Other
            }
        }
    };

    debug!(
        ?fetched_messages,
        ?notification_context,
        "process_fetched_messages"
    );

    match notification_context {
        NotificationContext::Intro | NotificationContext::ConversationList => {
            return; // suppress all notifications
        }
        NotificationContext::Conversation(conversation_id) => {
            // Remove notifications for the current conversation
            fetched_messages
                .notifications_content
                .retain(|notification| notification.conversation_id != Some(conversation_id));
        }
        NotificationContext::Other => (),
    }

    for notification in fetched_messages.notifications_content {
        notification_service.send_notification(notification).await;
    }
}
