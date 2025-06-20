// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{pin::pin, sync::Arc};

use flutter_rust_bridge::frb;
use phnxcoreclient::{
    AttachmentId,
    clients::CoreUser,
    store::{Store, StoreEntityId, StoreOperation},
};
use tokio_stream::StreamExt;
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::error;

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

#[frb(opaque)]
pub struct AttachmentsCubitBase {
    core: CubitCore<AttachmentsState>,
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
        spawn_attachment_downloads(store, cancel.clone());

        Self {
            core,
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
}

fn spawn_attachment_downloads(store: CoreUser, cancel: CancellationToken) {
    spawn_from_sync(attachment_downloads_loop(store, cancel));
}

async fn attachment_downloads_loop(store: CoreUser, cancel: CancellationToken) {
    let mut store_notifications = pin!(store.subscribe());
    loop {
        if cancel.is_cancelled() {
            return;
        }

        // download pending attachments
        match store.pending_attachments().await {
            Ok(pending_attachments) => {
                for attachment_id in pending_attachments {
                    spawn_download_task(store.clone(), cancel.clone(), attachment_id);
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

        // download newly added attachments
        for (id, ops) in &notification.ops {
            match id {
                StoreEntityId::Attachment(attachment_id) if ops.contains(StoreOperation::Add) => {
                    spawn_download_task(store.clone(), cancel.clone(), *attachment_id);
                }
                _ => (),
            }
        }
    }
}

fn spawn_download_task(store: CoreUser, cancel: CancellationToken, attachment_id: AttachmentId) {
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel.cancelled() => {},
            _ = store.download_attachment(attachment_id) => {}
        }
    });
}
