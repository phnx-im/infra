// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use aircoreclient::clients::{
    QueueEvent, QueueEventUpdate, process::process_qs::ProcessedQsMessages, queue_event,
};
use flutter_rust_bridge::frb;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    api::user::User,
    util::{BackgroundStreamContext, BackgroundStreamTask},
};

use super::{AppState, CubitContext};

#[derive(Debug)]
#[frb(ignore)]
pub(super) struct QueueContext {
    cubit_context: CubitContext,
}

impl BackgroundStreamContext<QueueEvent> for QueueContext {
    async fn create_stream(&self) -> anyhow::Result<impl Stream<Item = QueueEvent> + 'static> {
        let stream = self.cubit_context.core_user.listen_queue().await?;
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
                let core_user = self.cubit_context.core_user.clone();
                let user = User::from_core_user(core_user);
                match user.fetch_and_process_qs_messages().await {
                    Ok(ProcessedQsMessages {
                        new_conversations,
                        changed_conversations: _,
                        new_messages,
                        errors: _,
                    }) => {
                        let mut notifications =
                            Vec::with_capacity(new_conversations.len() + new_messages.len());
                        user.new_conversation_notifications(&new_conversations, &mut notifications)
                            .await;
                        user.new_message_notifications(&new_messages, &mut notifications)
                            .await;
                        self.cubit_context.show_notifications(notifications).await;
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
            .cubit_context
            .app_state
            .clone()
            .wait_for(|app_state| matches!(app_state, AppState::Foreground))
            .await;
    }

    async fn in_background(&self) {
        let _ = self
            .cubit_context
            .app_state
            .clone()
            .wait_for(|app_state| matches!(app_state, AppState::Background))
            .await;
    }
}

impl QueueContext {
    pub(super) fn new(cubit_context: CubitContext) -> Self {
        Self { cubit_context }
    }

    pub(super) fn into_task(
        self,
        cancel: CancellationToken,
    ) -> BackgroundStreamTask<Self, QueueEvent> {
        BackgroundStreamTask::new("qs", self, cancel)
    }
}
