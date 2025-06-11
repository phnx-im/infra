// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use phnxcoreclient::clients::{CoreUser, QueueEvent, QueueEventUpdate, queue_event};
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    api::{navigation_cubit::NavigationState, user::User},
    notifications::NotificationService,
    util::{BackgroundStreamContext, BackgroundStreamTask},
};

use super::AppState;

#[derive(Debug, Clone)]
#[frb(ignore)]
pub(super) struct QueueContext {
    core_user: CoreUser,
    navigation_state: watch::Receiver<NavigationState>,
    app_state: watch::Receiver<AppState>,
    notification_service: NotificationService,
}

impl BackgroundStreamContext<QueueEvent> for QueueContext {
    async fn create_stream(&self) -> anyhow::Result<impl Stream<Item = QueueEvent> + 'static> {
        let stream = self.core_user.listen_queue().await?;
        // Immediately emit an update event to kick off the initial state
        let initial_event = QueueEvent {
            event: Some(queue_event::Event::Update(QueueEventUpdate {})),
        };
        Ok(tokio_stream::once(initial_event).chain(stream))
    }

    async fn handle_event(&self, event: QueueEvent) {
        debug!(?event, "handling listen event");
        match event.event {
            Some(queue_event::Event::Payload(_)) => {
                // currently, we don't handle payload events
                warn!("ignoring listen event")
            }
            Some(queue_event::Event::Update(_)) => {
                let core_user = self.core_user.clone();
                let user = User::from_core_user(core_user);
                match user.fetch_all_messages().await {
                    Ok(fetched_messages) => {
                        super::process_fetched_messages(
                            &self.navigation_state,
                            &self.notification_service,
                            fetched_messages,
                        )
                        .await;
                    }
                    Err(error) => {
                        error!(%error, "failed to fetch messages on queue update");
                    }
                }
            }
            None => {}
        }
    }

    async fn in_foreground(&self) {
        let _ = self
            .app_state
            .clone()
            .wait_for(|app_state| matches!(app_state, AppState::Foreground))
            .await;
    }

    async fn in_background(&self) {
        let _ = self
            .app_state
            .clone()
            .wait_for(|app_state| matches!(app_state, AppState::Background))
            .await;
    }
}

impl QueueContext {
    pub(super) fn new(
        core_user: CoreUser,
        navigation_state: watch::Receiver<NavigationState>,
        app_state: watch::Receiver<AppState>,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            core_user,
            navigation_state,
            app_state,
            notification_service,
        }
    }

    pub(super) fn into_task(
        self,
        cancel: CancellationToken,
    ) -> BackgroundStreamTask<Self, QueueEvent> {
        BackgroundStreamTask::new("qs", self, cancel)
    }
}
