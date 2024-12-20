// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use phnxtypes::identifiers::QualifiedUserName;

use crate::{ConversationId, ConversationMessageId};

use super::StoreNotification;

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
