// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::types::NotificationType;

pub(crate) trait CoreClientProvider: Send + Sync {
    type NotificationProvider: NotificationProvider;
    fn notification_provider(&self) -> &Self::NotificationProvider;
}

pub(crate) trait NotificationProvider {
    fn notify(&self, notification_type: NotificationType) -> bool;
}
