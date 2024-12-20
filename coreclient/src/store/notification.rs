// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use log::error;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::StoreEntityId;

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

    pub(crate) fn notify_on_drop(&self) -> StoreNotificationGuard<'_> {
        StoreNotificationGuard {
            tx: self,
            builder: StoreNotificationBuilder::default(),
        }
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

pub(crate) struct StoreNotificationGuard<'a> {
    tx: &'a StoreNotificationsSender,
    builder: StoreNotificationBuilder,
}

impl Deref for StoreNotificationGuard<'_> {
    type Target = StoreNotificationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

impl DerefMut for StoreNotificationGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.builder
    }
}

impl Drop for StoreNotificationGuard<'_> {
    fn drop(&mut self) {
        if !self.builder.is_empty() {
            let mut builder = std::mem::take(&mut self.builder);
            self.tx.notify(builder.build());
        }
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
    pub(crate) fn add(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.inner.added.push(id.into());
        self
    }

    #[expect(dead_code)]
    pub(crate) fn add_many(
        &mut self,
        ids: impl IntoIterator<Item = impl Into<StoreEntityId>>,
    ) -> &mut Self {
        self.inner.added.extend(ids.into_iter().map(Into::into));
        self
    }

    pub(crate) fn update(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.inner.updated.push(id.into());
        self
    }

    pub(crate) fn update_many(
        &mut self,
        ids: impl IntoIterator<Item = impl Into<StoreEntityId>>,
    ) -> &mut Self {
        self.inner.updated.extend(ids.into_iter().map(Into::into));
        self
    }

    pub(crate) fn remove(&mut self, id: impl Into<StoreEntityId>) -> &mut Self {
        self.inner.removed.push(id.into());
        self
    }

    #[expect(dead_code)]
    pub(crate) fn remove_many(
        &mut self,
        ids: impl IntoIterator<Item = impl Into<StoreEntityId>>,
    ) -> &mut Self {
        self.inner.removed.extend(ids.into_iter().map(Into::into));
        self
    }

    pub(crate) fn build(&mut self) -> Arc<StoreNotification> {
        let mut inner = std::mem::take(&mut self.inner);
        inner.added.shrink_to_fit();
        inner.updated.shrink_to_fit();
        inner.removed.shrink_to_fit();
        inner.added.sort_unstable();
        inner.updated.sort_unstable();
        inner.removed.sort_unstable();
        Arc::new(inner)
    }

    fn is_empty(&self) -> bool {
        self.inner.added.is_empty()
            && self.inner.updated.is_empty()
            && self.inner.removed.is_empty()
    }
}

impl From<&mut StoreNotificationBuilder> for Arc<StoreNotification> {
    fn from(builder: &mut StoreNotificationBuilder) -> Self {
        builder.build()
    }
}
