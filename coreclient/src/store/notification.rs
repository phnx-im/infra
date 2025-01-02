// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{mem, sync::Arc};

use log::{error, warn};
use phnxtypes::identifiers::QualifiedUserName;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tokio_stream::{Stream, StreamExt};

use crate::{ConversationId, ConversationMessageId};

// 1024 * size_of::<Arc<StoreNotification>>() = 1024 * 8 = 8 KiB
const NOTIFICATION_CHANNEL_SIZE: usize = 1024;

pub(crate) struct StoreNotifier {
    tx: StoreNotificationsSender,
    notification: StoreNotification,
}

impl StoreNotifier {
    pub(crate) fn new(tx: StoreNotificationsSender) -> Self {
        Self {
            tx,
            notification: Default::default(),
        }
    }

    pub(crate) fn noop() -> Self {
        Self {
            tx: StoreNotificationsSender::new(),
            notification: Default::default(),
        }
    }

    pub(crate) fn add(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.notification.added.push(id.into());
        self
    }

    pub(crate) fn update(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.notification.updated.push(id.into());
        self
    }

    pub(crate) fn remove(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.notification.removed.push(id.into());
        self
    }

    pub(crate) fn notify(mut self) {
        if !self.notification.is_empty() {
            let mut notification = mem::take(&mut self.notification);

            notification.added.shrink_to_fit();
            notification.updated.shrink_to_fit();
            notification.removed.shrink_to_fit();
            notification.added.sort_unstable();
            notification.updated.sort_unstable();
            notification.removed.sort_unstable();

            self.tx.notify(Arc::new(notification));
        }
    }
}

impl Drop for StoreNotifier {
    fn drop(&mut self) {
        if !self.notification.is_empty() {
            // Note: This might be ok. E.g. an error might happen after some notifications were
            // added to the notifier.
            warn!(
                "StoreNotifier dropped with notifications; did you forget to call notify()? notifications = {:?}",
                self.notification
            );
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoreNotificationsSender {
    tx: broadcast::Sender<Arc<StoreNotification>>,
}

impl StoreNotificationsSender {
    pub(crate) fn new() -> Self {
        let (tx, _) = broadcast::channel(NOTIFICATION_CHANNEL_SIZE);
        Self { tx }
    }

    pub(crate) fn notify(&self, notification: impl Into<Arc<StoreNotification>>) {
        let _no_receivers = self.tx.send(notification.into());
    }

    pub(crate) fn subscribe(&self) -> impl Stream<Item = Arc<StoreNotification>> {
        BroadcastStream::new(self.tx.subscribe()).filter_map(|res| match res {
            Ok(notification) => Some(notification),
            Err(BroadcastStreamRecvError::Lagged(n)) => {
                error!("store notifications lagged by {} messages", n);
                None
            }
        })
    }
}

impl Default for StoreNotificationsSender {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Clone)]
pub struct StoreNotification {
    pub added: Vec<StoreEntityId>,
    pub updated: Vec<StoreEntityId>,
    pub removed: Vec<StoreEntityId>,
}

// Note(perf): I would prefer this type to be copy and smaller in memory (currently 48 bytes), but
// `QualifiedUserName` is not copy and quite large.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
pub enum StoreEntityId {
    OwnUser,
    User(QualifiedUserName),
    Conversation(ConversationId),
    Message(ConversationMessageId),
}

impl StoreNotification {
    pub fn contains_added(&self, id: &StoreEntityId) -> bool {
        self.added.binary_search(id).is_ok()
    }

    pub fn contains_updated(&self, id: &StoreEntityId) -> bool {
        self.updated.binary_search(id).is_ok()
    }

    pub fn contains_removed(&self, id: &StoreEntityId) -> bool {
        self.removed.binary_search(id).is_ok()
    }

    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }
}
