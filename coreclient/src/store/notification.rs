// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use phnxtypes::identifiers::QualifiedUserName;

use crate::{ConversationId, ConversationMessageId};

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
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add(&mut self, id: impl Into<StoreEntityId>) {
        self.inner.added.push(id.into());
    }

    pub(crate) fn update(&mut self, id: impl Into<StoreEntityId>) {
        self.inner.added.push(id.into());
    }

    pub(crate) fn remove(&mut self, id: impl Into<StoreEntityId>) {
        self.inner.added.push(id.into());
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
