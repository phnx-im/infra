// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{HashMap, hash_map},
    pin::pin,
    sync::Arc,
};

use anyhow::bail;
use flutter_rust_bridge::frb;
use phnxcommon::identifiers::AttachmentId;
use phnxcoreclient::{
    AttachmentContent, DownloadProgress,
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{debug, error, info};

use crate::{
    StreamSink,
    api::user_cubit::UserCubitBase,
    util::{Cubit, CubitCore, spawn_from_sync},
};

#[derive(Debug, Clone)]
#[frb(opaque)]
#[allow(dead_code)]
pub struct AttachmentsState {
    inner: Arc<AttachmentsStateInner>,
}

#[frb(ignore)]
#[derive(Debug, Clone)]
struct AttachmentsStateInner {}

impl AttachmentsStateInner {
    fn new() -> Self {
        Self {}
    }
}

type InProgressMap = Arc<Mutex<HashMap<AttachmentId, DownloadTaskHandle>>>;

#[frb(opaque)]
pub struct AttachmentsCubitBase {
    core: CubitCore<AttachmentsState>,
    store: CoreUser,
    cancel: CancellationToken,
    in_progress: InProgressMap,
    _cancel: DropGuard,
}

impl AttachmentsCubitBase {
    #[frb(sync)]
    pub fn new(user_cubit: &UserCubitBase) -> Self {
        let store = user_cubit.core_user().clone();

        let inner = AttachmentsStateInner::new();
        let core = CubitCore::with_initial_state(AttachmentsState {
            inner: Arc::new(inner),
        });

        let cancel = CancellationToken::new();
        let in_progress = InProgressMap::default();
        spawn_attachment_downloads(store.clone(), in_progress.clone(), cancel.clone());

        Self {
            core,
            store,
            in_progress,
            cancel: cancel.clone(),
            _cancel: cancel.drop_guard(),
        }
    }

    // Cubit interface

    pub fn close(&mut self) {
        self.core.close();
    }

    #[frb(getter, sync)]
    pub fn is_closed(&self) -> bool {
        self.core.is_closed()
    }

    #[frb(getter, sync)]
    pub fn state(&self) -> AttachmentsState {
        self.core.state()
    }

    pub async fn stream(&mut self, sink: StreamSink<AttachmentsState>) {
        self.core.stream(sink).await;
    }

    // Cubit methods

    pub async fn load_attachment(&self, attachment_id: AttachmentId) -> anyhow::Result<Vec<u8>> {
        let mut content = None;
        loop {
            let loaded_content = match content.take() {
                Some(loaded_content) => loaded_content,
                None => self.store.load_attachment(attachment_id).await?,
            };
            match loaded_content {
                AttachmentContent::None => bail!("Attachment not found"),
                AttachmentContent::Ready(bytes) => return Ok(bytes),
                AttachmentContent::Pending => {
                    spawn_download_task(
                        &self.store,
                        &mut *self.in_progress.lock().await,
                        &self.cancel,
                        attachment_id,
                    );
                    content = Some(AttachmentContent::Downloading);
                }
                AttachmentContent::Downloading => {
                    // wait for download to complete
                    let handle = self.in_progress.lock().await.get(&attachment_id).cloned();
                    if let Some(mut handle) = handle {
                        handle.progress.wait_for_completion().await;
                    }
                }
                AttachmentContent::Failed | AttachmentContent::Unknown => {
                    bail!("Attachment download failed")
                }
            }
        }
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

    let mut store_notifications = pin!(store.subscribe());
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
) {
    let (task, cancel) = match in_progress.entry(attachment_id) {
        hash_map::Entry::Occupied(mut entry) if entry.get().cancel.is_cancelled() => {
            let (progress, task) = store.download_attachment(attachment_id);
            let cancel = cancel.child_token();
            entry.insert(DownloadTaskHandle {
                progress,
                cancel: cancel.clone(),
                _drop_guard: Arc::new(cancel.clone().drop_guard()),
            });
            (task, cancel)
        }
        hash_map::Entry::Vacant(entry) => {
            let (progress, task) = store.download_attachment(attachment_id);
            let cancel = cancel.child_token();
            entry.insert(DownloadTaskHandle {
                progress,
                cancel: cancel.clone(),
                _drop_guard: Arc::new(cancel.clone().drop_guard()),
            });
            (task, cancel)
        }
        _ => return, // already in progress
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
}

#[derive(Debug, Clone)]
struct DownloadTaskHandle {
    progress: DownloadProgress,
    cancel: CancellationToken,
    _drop_guard: Arc<DropGuard>,
}
