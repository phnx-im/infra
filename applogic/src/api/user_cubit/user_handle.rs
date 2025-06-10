// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, convert::identity, sync::Arc};

use anyhow::{Context, bail};
use phnxcommon::identifiers::UserHandle;
use phnxcoreclient::{
    UserHandleRecord,
    clients::{HandleQueueMessage, ListenHandleResponder},
    store::Store,
};
use tokio::sync::{RwLock, watch};
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{debug, error};
use uuid::Uuid;

use crate::util::{BackgroundStreamContext, BackgroundStreamTask, spawn_from_sync};

use super::qs::QueueContext;

/// The context of the background task that listens to a user handle.
#[derive(Debug, Clone)]
pub(super) struct HandleContext {
    queue_context: QueueContext,
    handle_record: Arc<UserHandleRecord>,
    responder: Arc<RwLock<Option<ListenHandleResponder>>>,
}

impl HandleContext {
    pub(super) fn new(queue_context: QueueContext, handle_record: UserHandleRecord) -> Self {
        Self {
            queue_context,
            handle_record: Arc::new(handle_record),
            responder: Default::default(),
        }
    }

    /// Spawns a task that loads all user handle records in the background and spawns a new listen
    /// handle background task for each record.
    pub(super) fn spawn_loading(
        queue_context: QueueContext,
        parent_cancel: CancellationToken,
        handle_background_tasks: HandleBackgroundTasks,
    ) {
        spawn_from_sync(async move {
            let records = match queue_context.core_user.user_handle_records().await {
                Ok(records) => records,
                Err(error) => {
                    error!(%error, "failed to load user handle records; won't listen to handles");
                    return;
                }
            };
            for record in records {
                Self::new(queue_context.clone(), record)
                    .into_task(parent_cancel.child_token(), &handle_background_tasks)
                    .spawn();
            }
        });
    }

    pub(super) fn into_task(
        self,
        cancel: CancellationToken,
        background_tasks: &HandleBackgroundTasks,
    ) -> BackgroundStreamTask<Self, HandleQueueMessage> {
        let handle = self.handle_record.handle.clone();
        let name = format!("handle-{}", handle.plaintext());
        background_tasks.insert(handle, cancel.clone());
        BackgroundStreamTask::new(name, self, cancel)
    }

    async fn ack(&self, message_id: Option<Uuid>) {
        if let Err(error) = self.try_ack(message_id).await {
            error!(%error, "failed to ack handle queue message");
        }
    }

    async fn try_ack(&self, message_id: Option<Uuid>) -> anyhow::Result<()> {
        let message_id = message_id.context("no message id in handle queue message")?;
        let response = self.responder.read().await;
        let Some(responder) = response.as_ref() else {
            bail!("logic error: no handle queue responder");
        };
        debug!(?message_id, "acking handle queue message");
        responder.ack(message_id).await;
        Ok(())
    }
}

impl BackgroundStreamContext<HandleQueueMessage> for HandleContext {
    async fn in_foreground(&self) {
        self.queue_context.in_foreground().await;
    }

    async fn in_background(&self) {
        self.queue_context.in_background().await;
    }

    async fn create_stream(&self) -> anyhow::Result<impl Stream<Item = HandleQueueMessage>> {
        let (stream, responder) = self
            .queue_context
            .core_user
            .listen_handle(self.handle_record.hash, &self.handle_record.signing_key)
            .await?;
        self.responder.write().await.replace(responder);
        Ok(stream.filter_map(identity))
    }

    async fn handle_event(&self, message: HandleQueueMessage) {
        let message_id = message.message_id.map(From::from);
        if let Err(error) = self
            .queue_context
            .core_user
            .process_handle_queue_message(&self.handle_record.handle.clone(), message)
            .await
        {
            error!(?error, "failed to process handle queue message");
        }
        // ack the message independently of the result of processing the message
        self.ack(message_id).await;
    }
}

/// Tracks the background tasks listening to user handles.
#[derive(Debug, Clone)]
pub(super) struct HandleBackgroundTasks {
    tx: watch::Sender<HashMap<UserHandle, DropGuard>>,
}

impl HandleBackgroundTasks {
    pub(super) fn new() -> Self {
        Self {
            tx: watch::channel(Default::default()).0,
        }
    }

    pub(super) fn insert(&self, handle: UserHandle, cancel: CancellationToken) {
        self.tx.send_modify(|handles| {
            handles.insert(handle, cancel.drop_guard());
        });
    }

    pub(super) fn remove(&self, handle: UserHandle) {
        self.tx.send_modify(|handles| {
            handles.remove(&handle);
        });
    }
}

impl Default for HandleBackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}
