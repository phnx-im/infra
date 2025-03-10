// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::BTreeMap, mem, sync::Arc};

use phnxtypes::identifiers::QualifiedUserName;
use tokio::sync::broadcast;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
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
        let notification = notification.into();
        debug!(
            num_receivers = self.tx.receiver_count(),
            ?notification,
            "StoreNotificationsSender::notify"
        );
        let _no_receivers = self.tx.send(notification);
    }

    /// Creates a new subscription to the notifications.
    ///
    /// The stream will contain all notifications from the moment this function is called.
    pub(crate) fn subscribe(&self) -> impl Stream<Item = Arc<StoreNotification>> + 'static {
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

    /// Returns all pending notifications.
    ///
    /// The pending notifications are the notifications captured starting at the call to this function.
    /// Getting the next item from the iterator gets the next pending notification is there is any,
    /// otherwise it returns `None`. Therefore, the iterator is not fused.
    ///
    /// This is useful for capturing all pending notifications synchronously.
    pub(crate) fn subscribe_iter(
        &self,
    ) -> impl Iterator<Item = Arc<StoreNotification>> + Send + 'static {
        let mut rx = self.tx.subscribe();
        std::iter::from_fn(move || {
            loop {
                match rx.try_recv() {
                    Ok(notification) => return Some(notification),
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        error!(n, "store notifications lagged");
                        continue;
                    }
                    Err(
                        broadcast::error::TryRecvError::Closed
                        | broadcast::error::TryRecvError::Empty,
                    ) => return None,
                }
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
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct StoreNotification {
    pub ops: BTreeMap<StoreEntityId, StoreOperation>,
}

impl StoreNotification {
    fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
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

impl StoreEntityId {
    pub(crate) fn kind(&self) -> StoreEntityKind {
        match self {
            StoreEntityId::User(_) => StoreEntityKind::User,
            StoreEntityId::Conversation(_) => StoreEntityKind::Conversation,
            StoreEntityId::Message(_) => StoreEntityKind::Message,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum StoreEntityKind {
    User = 0,
    Conversation = 1,
    Message = 2,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid store entity kind: {0}")]
pub(crate) struct InvalidStoreEntityKind(i64);

impl TryFrom<i64> for StoreEntityKind {
    type Error = InvalidStoreEntityKind;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(StoreEntityKind::User),
            1 => Ok(StoreEntityKind::Conversation),
            2 => Ok(StoreEntityKind::Message),
            _ => Err(InvalidStoreEntityKind(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_iter() {
        let tx = StoreNotificationsSender::new();

        let ops_1: BTreeMap<_, _> = [(
            StoreEntityId::User("alice@localhost".parse().unwrap()),
            StoreOperation::Add,
        )]
        .into_iter()
        .collect();

        let ops_2: BTreeMap<_, _> = [(
            StoreEntityId::User("bob@localhost".parse().unwrap()),
            StoreOperation::Update,
        )]
        .into_iter()
        .collect();

        let ops_3: BTreeMap<_, _> = [(
            StoreEntityId::User("eve@localhost".parse().unwrap()),
            StoreOperation::Remove,
        )]
        .into_iter()
        .collect();

        let ops_4: BTreeMap<_, _> = [(
            StoreEntityId::User("mallory@localhost".parse().unwrap()),
            StoreOperation::Add,
        )]
        .into_iter()
        .collect();

        tx.notify(StoreNotification {
            ops: ops_1.into_iter().collect(),
        });

        let mut iter = tx.subscribe_iter();

        tx.notify(StoreNotification { ops: ops_2.clone() });

        // first notification is not observed, because it was sent before the subscription
        assert_eq!(iter.next().unwrap().ops, ops_2);
        assert_eq!(iter.next(), None);

        tx.notify(StoreNotification { ops: ops_3.clone() });
        assert_eq!(iter.next().unwrap().ops, ops_3);
        tx.notify(StoreNotification { ops: ops_4.clone() });
        assert_eq!(iter.next().unwrap().ops, ops_4);
        assert_eq!(iter.next(), None);
    }
}
