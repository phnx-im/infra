// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg_attr(
    any(target_os = "linux", target_os = "windows"),
    expect(
        dead_code,
        reason = "DartNotificationService is only used on iOS/macOS and Android"
    )
)]

use std::{fmt, sync::Arc};

use flutter_rust_bridge::{DartFnFuture, frb};
use tracing::debug;

pub use crate::notifications::{NotificationContent, NotificationHandle, NotificationId};

#[frb(opaque)]
#[derive(Clone)]
pub struct DartNotificationService {
    callback: Arc<Callbacks>,
}

impl fmt::Debug for DartNotificationService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DartNotificationService")
            .finish_non_exhaustive()
    }
}

#[frb(ignore)]
struct Callbacks {
    send: Box<dyn Fn(NotificationContent) -> DartFnFuture<()> + Send + Sync + 'static>,
    get_active: Box<dyn Fn() -> DartFnFuture<Vec<NotificationHandle>> + Send + Sync + 'static>,
    cancel: Box<dyn Fn(Vec<NotificationId>) -> DartFnFuture<()> + Send + Sync + 'static>,
}

impl DartNotificationService {
    #[frb(sync)]
    pub fn new(
        send: impl Fn(NotificationContent) -> DartFnFuture<()> + Send + Sync + 'static,
        get_active: impl Fn() -> DartFnFuture<Vec<NotificationHandle>> + Send + Sync + 'static,
        cancel: impl Fn(Vec<NotificationId>) -> DartFnFuture<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            callback: Arc::new(Callbacks {
                send: Box::new(send),
                get_active: Box::new(get_active),
                cancel: Box::new(cancel),
            }),
        }
    }

    pub(crate) async fn send_notification(&self, notification: NotificationContent) {
        debug!(?notification, "send notification over dart service");
        (self.callback.send)(notification).await;
    }

    pub(crate) async fn get_active_notifications(&self) -> Vec<NotificationHandle> {
        debug!("get active notifications over dart service");
        (self.callback.get_active)().await
    }

    pub(crate) async fn cancel_notifications(&self, identifiers: Vec<NotificationId>) {
        debug!(?identifiers, "cancel notifications over dart service");
        (self.callback.cancel)(identifiers).await;
    }
}
