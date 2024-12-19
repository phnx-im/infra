// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use log::error;
use phnxtypes::identifiers::QualifiedUserName;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::{ConversationId, ConversationMessageId};

// 1024 * size_of::<Arc<StoreNotification>>() = 1024 * 8 = 8 KiB
const NOTIFICATION_CHANNEL_SIZE: usize = 1024;

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

    pub(crate) fn subscribe(
        &self,
    ) -> impl tokio_stream::Stream<Item = std::sync::Arc<StoreNotification>> {
        BroadcastStream::new(self.tx.subscribe()).map(|res| match res {
            Ok(notification) => notification,
            Err(BroadcastStreamRecvError::Lagged(n)) => {
                error!("store notifications lagged by {} messages", n);
                std::sync::Arc::new(StoreNotification::default())
            }
        })
    }
}

impl Default for StoreNotificationsSender {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
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

impl StoreEntityId {
    pub(crate) fn added(self) -> Arc<StoreNotification> {
        StoreNotification::builder().add(self).build()
    }

    pub(crate) fn updated(self) -> Arc<StoreNotification> {
        StoreNotification::builder().update(self).build()
    }

    pub(crate) fn removed(self) -> Arc<StoreNotification> {
        StoreNotification::builder().remove(self).build()
    }
}

impl StoreNotification {
    pub(crate) fn builder() -> StoreNotificationBuilder {
        StoreNotificationBuilder::default()
    }

    pub fn contains_added(&self, id: &StoreEntityId) -> bool {
        self.added.binary_search(id).is_ok()
    }

    pub fn contains_updated(&self, id: &StoreEntityId) -> bool {
        self.updated.binary_search(id).is_ok()
    }

    pub fn contains_removed(&self, id: &StoreEntityId) -> bool {
        self.removed.binary_search(id).is_ok()
    }
}

#[derive(Debug, Default)]
pub struct StoreNotificationBuilder {
    inner: StoreNotification,
}

impl StoreNotificationBuilder {
    pub(crate) fn add(mut self, id: impl Into<StoreEntityId>) -> Self {
        self.inner.added.push(id.into());
        self
    }

    #[expect(dead_code)]
    pub(crate) fn add_many(
        mut self,
        ids: impl IntoIterator<Item = impl Into<StoreEntityId>>,
    ) -> Self {
        self.inner.added.extend(ids.into_iter().map(Into::into));
        self
    }

    pub(crate) fn update(mut self, id: impl Into<StoreEntityId>) -> Self {
        self.inner.updated.push(id.into());
        self
    }

    pub(crate) fn update_many(
        mut self,
        ids: impl IntoIterator<Item = impl Into<StoreEntityId>>,
    ) -> Self {
        self.inner.updated.extend(ids.into_iter().map(Into::into));
        self
    }

    pub(crate) fn remove(mut self, id: impl Into<StoreEntityId>) -> Self {
        self.inner.removed.push(id.into());
        self
    }

    #[expect(dead_code)]
    pub(crate) fn remove_many(
        mut self,
        ids: impl IntoIterator<Item = impl Into<StoreEntityId>>,
    ) -> Self {
        self.inner.removed.extend(ids.into_iter().map(Into::into));
        self
    }

    pub(crate) fn build(self) -> Arc<StoreNotification> {
        let mut inner = self.inner;
        inner.added.shrink_to_fit();
        inner.updated.shrink_to_fit();
        inner.removed.shrink_to_fit();
        inner.added.sort_unstable();
        inner.updated.sort_unstable();
        inner.removed.sort_unstable();
        Arc::new(inner)
    }
}

impl From<StoreNotificationBuilder> for Arc<StoreNotification> {
    fn from(builder: StoreNotificationBuilder) -> Self {
        builder.build()
    }
}
