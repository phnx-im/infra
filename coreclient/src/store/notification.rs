// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::BTreeMap, mem, sync::Arc};

use phnxtypes::identifiers::QualifiedUserName;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tokio_stream::{Stream, StreamExt};
use tracing::{debug, error, warn};

use crate::{ConversationId, ConversationMessageId};

// 1024 * size_of::<Arc<StoreNotification>>() = 1024 * 8 = 8 KiB
const NOTIFICATION_CHANNEL_SIZE: usize = 1024;

/// Bundles a notification sender and a notification.
///
/// Used to collect all notifications and eventually send them all at once.
pub(crate) struct StoreNotifier {
    tx: StoreNotificationsSender,
    notification: StoreNotification,
}

impl StoreNotifier {
    /// Creates a new notifier which will send all notifications with the given sender.
    pub(crate) fn new(tx: StoreNotificationsSender) -> Self {
        Self {
            tx,
            notification: StoreNotification::empty(),
        }
    }

    /// Creates a new notifier which will just drop all notifications.
    ///
    /// Useful when a notifier is required, but no notifications are actually should be emitted.
    pub(crate) fn noop() -> Self {
        Self {
            tx: StoreNotificationsSender::new(),
            notification: StoreNotification::empty(),
        }
    }

    /// Add a new entity to the notification.
    ///
    /// Notification will be sent when the `notify` function is called.
    pub(crate) fn add(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.notification.ops.insert(id.into(), StoreOperation::Add);
        self
    }

    /// Update an existing entity in the notification.
    ///
    /// Notification will be sent when the `notify` function is called.
    pub(crate) fn update(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.notification
            .ops
            .insert(id.into(), StoreOperation::Update);
        self
    }

    /// Remove an existing entity from the notification.
    ///
    /// Notification will be sent when the `notify` function is called.
    pub(crate) fn remove(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.notification
            .ops
            .insert(id.into(), StoreOperation::Remove);
        self
    }

    /// Send collected notifications to the subscribers, if there are any.
    pub(crate) fn notify(mut self) {
        if !self.notification.ops.is_empty() {
            let notification = mem::replace(&mut self.notification, StoreNotification::empty());
            self.tx.notify(Arc::new(notification));
        }
    }
}

impl Drop for StoreNotifier {
    fn drop(&mut self) {
        if !self.notification.ops.is_empty() {
            // Note: This might be ok. E.g. an error might happen after some notifications were
            // added to the notifier.
            warn!(
                "StoreNotifier dropped with notifications; \
                    did you forget to call notify()? notifications = {:?}",
                self.notification
            );
        }
    }
}

/// A channel for sending or subscribing to notifications
#[derive(Debug, Clone)]
pub(crate) struct StoreNotificationsSender {
    tx: broadcast::Sender<Arc<StoreNotification>>,
}

impl StoreNotificationsSender {
    /// Createa a new notification sender without any subscribers.
    pub(crate) fn new() -> Self {
        let (tx, _) = broadcast::channel(NOTIFICATION_CHANNEL_SIZE);
        Self { tx }
    }

    /// Sends a notification to all current subscribers.
    pub(crate) fn notify(&self, notification: impl Into<Arc<StoreNotification>>) {
        let _no_receivers = self.tx.send(notification.into());
    }

    /// Creates a new subscription to the notifications.
    ///
    /// The stream will contain all notifications from the moment this function is called.
    pub(crate) fn subscribe(&self) -> impl Stream<Item = Arc<StoreNotification>> {
        BroadcastStream::new(self.tx.subscribe()).filter_map(|res| match res {
            Ok(notification) => {
                debug!(?notification, "Received store notification");
                Some(notification)
            }
            Err(BroadcastStreamRecvError::Lagged(n)) => {
                error!(n, "store notifications lagged");
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

/// A store notification bundle
///
/// Bundles all changes to the store, that is, all entities that have been added, updated or
/// removed.
#[derive(Debug, Default)]
pub struct StoreNotification {
    pub ops: BTreeMap<StoreEntityId, StoreOperation>,
}

impl StoreNotification {
    fn empty() -> Self {
        Self::default()
    }
}

/// Operation which was performed in a [`super::Store`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StoreOperation {
    Add,
    Update,
    Remove,
}

/// Identifier of an enitity of a [`super::Store`].
///
/// Used to identify added, updated or removed entites in a [`StoreNotification`].
// Note(perf): I would prefer this type to be copy and smaller in memory (currently 48 bytes), but
// `QualifiedUserName` is not copy and quite large.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
pub enum StoreEntityId {
    User(QualifiedUserName),
    Conversation(ConversationId),
    Message(ConversationMessageId),
}
