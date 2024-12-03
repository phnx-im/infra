// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[cfg(target_os = "android")]
pub mod java_api;

#[cfg(target_os = "ios")]
pub mod swift_api;

#[cfg(any(target_os = "ios", target_os = "android"))]
pub(crate) mod processing;

#[derive(Serialize, Deserialize)]
pub(crate) struct IncomingNotificationContent {
    title: String,
    body: String,
    data: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NotificationBatch {
    badge_count: u32,
    removals: Vec<String>,
    additions: Vec<NotificationContent>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NotificationContent {
    identifier: String,
    title: String,
    body: String,
    data: String,
}
