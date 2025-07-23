// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, hash_map},
    sync::Arc,
};

use anyhow::{Context, bail};
use flutter_rust_bridge::{DartFnFuture, frb};
use phnxcommon::identifiers::AttachmentId;
use phnxcoreclient::{
    AttachmentContent, DownloadProgress, DownloadProgressEvent,
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{debug, error, info};

use crate::{api::user_cubit::UserCubitBase, util::spawn_from_sync};

type InProgressMap = Arc<Mutex<HashMap<AttachmentId, DownloadTaskHandle>>>;

/// Repository managing attachments
///
/// * Listens to store notifications and spawns download tasks for attachments that are added or
/// pending.
/// * Provides access for loading attachments.
#[frb(opaque)]
pub struct AttachmentsRepository {
    store: CoreUser,
    cancel: CancellationToken,
    in_progress: InProgressMap,
    _cancel: DropGuard,
}

impl AttachmentsRepository {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let store = user_cubit.core_user().clone();

        let cancel = CancellationToken::new();
        let in_progress = InProgressMap::default();
        spawn_attachment_downloads(store.clone(), in_progress.clone(), cancel.clone());

        Self {
            store,
            in_progress,
            cancel: cancel.clone(),
            _cancel: cancel.drop_guard(),
        }
    }

    pub async fn load_image_attachment(
        &self,
        attachment_id: AttachmentId,
        chunk_event_callback: impl Fn(u64) -> DartFnFuture<()> + Send + 'static,
    ) -> anyhow::Result<Vec<u8>> {
        match self.store.load_attachment(attachment_id).await? {
            AttachmentContent::Ready(bytes) => Ok(bytes),
            AttachmentContent::Pending => {
                debug!(?attachment_id, "Attachment is pending; spawn download task");
                let handle = spawn_download_task(
                    &self.store,
                    &mut *self.in_progress.lock().await,
                    &self.cancel,
                    attachment_id,
                );
                self.track_attachment_download(attachment_id, handle, chunk_event_callback)
                    .await
            }
            AttachmentContent::Downloading => {
                let handle = self.in_progress.lock().await.get(&attachment_id).cloned();
                if let Some(handle) = handle {
                    self.track_attachment_download(attachment_id, handle, chunk_event_callback)
                        .await
                } else {
                    match self.store.load_attachment(attachment_id).await? {
                        AttachmentContent::Ready(bytes) => Ok(bytes),
                        _ => bail!("Attachment download failed"),
                    }
                }
            }
            AttachmentContent::None => bail!("Attachment not found"),
            AttachmentContent::Failed | AttachmentContent::Unknown => {
                bail!("Attachment download failed")
            }
        }
    }

    async fn track_attachment_download(
        &self,
        attachment_id: AttachmentId,
        handle: DownloadTaskHandle,
        chunk_event_callback: impl Fn(u64) -> DartFnFuture<()> + Send + 'static,
    ) -> anyhow::Result<Vec<u8>> {
        debug!(?attachment_id, "Tracking attachment download");
        let mut events_stream = handle.progress.stream();
        while let Some(event) = events_stream.next().await {
            match event {
                DownloadProgressEvent::Init => {
                    chunk_event_callback(0).await;
                }
                DownloadProgressEvent::Progress { bytes_loaded } => {
                    chunk_event_callback(bytes_loaded.try_into()?).await;
                }
                DownloadProgressEvent::Completed => {
                    return self
                        .store
                        .load_attachment(attachment_id)
                        .await?
                        .into_bytes()
                        .context("Attachment download failed");
                }
                DownloadProgressEvent::Failed => bail!("Attachment download failed"),
            }
        }
        bail!("Attachment download aborted")
    }
}

fn spawn_attachment_downloads(
    store: CoreUser,
    in_progress: InProgressMap,

    cancel: CancellationToken,
) {
    spawn_from_sync(attachment_downloads_loop(store, in_progress, cancel));
}

async fn attachment_downloads_loop(
    store: CoreUser,
    in_progress: InProgressMap,
    cancel: CancellationToken,
) {
    info!("Starting attachments download loop");

    let mut store_notifications = store.subscribe();
    loop {
        if cancel.is_cancelled() {
            return;
        }

        // download pending attachments
        match store.pending_attachments().await {
            Ok(pending_attachments) => {
                debug!(
                    ?pending_attachments,
                    "Spawn download for pending attachments"
                );
                let mut in_progress = in_progress.lock().await;
                for attachment_id in pending_attachments {
                    spawn_download_task(&store, &mut in_progress, &cancel, attachment_id);
                }
            }
            Err(error) => {
                error!(%error, "Failed to load pending attachments");
            }
        }

        // wait for the next store notification
        let notification = tokio::select! {
            _ = cancel.cancelled() => return,
            notification = store_notifications.next() => notification,
        };
        let Some(notification) = notification else {
            return;
        };

        debug!(?notification, "Received store notification");

        // download newly added attachments
        for (id, ops) in &notification.ops {
            match id {
                StoreEntityId::Attachment(attachment_id) if ops.contains(StoreOperation::Add) => {
                    debug!(?attachment_id, "Spawn download for added attachment");
                    let mut in_progress = in_progress.lock().await;
                    spawn_download_task(&store, &mut in_progress, &cancel, *attachment_id);
                }
                _ => (),
            }
        }
    }
}

fn spawn_download_task(
    store: &CoreUser,
    in_progress: &mut HashMap<AttachmentId, DownloadTaskHandle>,
    cancel: &CancellationToken,
    attachment_id: AttachmentId,
) -> DownloadTaskHandle {
    let (task, cancel, handle) = match in_progress.entry(attachment_id) {
        hash_map::Entry::Occupied(mut entry) if entry.get().cancel.is_cancelled() => {
            let (progress, task) = store.download_attachment(attachment_id);
            let cancel = cancel.child_token();
            let handle = DownloadTaskHandle {
                progress,
                cancel: cancel.clone(),
                _drop_guard: Arc::new(cancel.clone().drop_guard()),
            };
            entry.insert(handle.clone());
            (task, cancel, handle)
        }
        hash_map::Entry::Occupied(entry) => {
            return entry.get().clone();
        }
        hash_map::Entry::Vacant(entry) => {
            let (progress, task) = store.download_attachment(attachment_id);
            let cancel = cancel.child_token();
            let handle = DownloadTaskHandle {
                progress,
                cancel: cancel.clone(),
                _drop_guard: Arc::new(cancel.clone().drop_guard()),
            };
            entry.insert(handle.clone());
            (task, cancel, handle)
        }
    };

    tokio::spawn(async move {
        tokio::select! {
            _ = cancel.cancelled() => {},
            res = task => {
                if let Err(error) = res {
                    error!(%error, "Failed to download attachment");
                }
                cancel.cancel(); // mark as done
            }
        }
    });

    handle
}

#[derive(Debug, Clone)]
struct DownloadTaskHandle {
    progress: DownloadProgress,
    cancel: CancellationToken,
    _drop_guard: Arc<DropGuard>,
}
