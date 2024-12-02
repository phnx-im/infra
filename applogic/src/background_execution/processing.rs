// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::panic::{self, AssertUnwindSafe};
use tokio::runtime::Builder;

use crate::api::user::User;

use super::{NotificationBatch, NotificationContent};

/// TODO: Debug code to be removed
pub(crate) fn error_batch(e: String) -> NotificationBatch {
    NotificationBatch {
        badge_count: 0,
        removals: Vec::new(),
        additions: vec![NotificationContent {
            identifier: "".to_string(),
            title: "Error".to_string(),
            body: e,
            data: "".to_string(),
        }],
    }
}

/// Wraps with a tokio runtime to block on the async functions
pub(crate) fn retrieve_messages_sync(path: String) -> NotificationBatch {
    let result = Builder::new_multi_thread()
        .thread_name("nse-thread")
        .enable_all()
        .build()
        .map_err(|e| {
            log::error!("Failed to initialize tokio runtime: {}", e);
            e.to_string()
        })
        .and_then(|runtime| {
            panic::catch_unwind(AssertUnwindSafe(|| {
                runtime.block_on(async { retrieve_messages(path).await })
            }))
            .map_err(|_| {
                let e = "Failed to execute async function".to_string();
                log::error!("{}", e);
                e
            })
        });

    match result {
        Ok(batch) => batch,
        Err(e) => error_batch(e),
    }
}

/// Load the user and retrieve messages
pub(crate) async fn retrieve_messages(path: String) -> NotificationBatch {
    log::info!("Retrieving messages with DB path: {}", path);
    let user = match User::load_default(path).await {
        Ok(user) => user,
        Err(e) => {
            log::error!("Failed to load user: {}", e);
            return error_batch(e.to_string());
        }
    };

    let notifications = match user.fetch_all_messages().await {
        Ok(fetched_messages) => {
            log::info!("All messages fetched");
            fetched_messages
                .notifications_content
                .into_iter()
                .map(|m| NotificationContent {
                    title: m.title,
                    body: m.body,
                    identifier: "".to_string(),
                    data: "".to_string(),
                })
                .collect()
        }
        Err(e) => vec![NotificationContent {
            identifier: "".to_string(),
            title: "Error fetching messages".to_string(),
            body: e.to_string(),
            data: "".to_string(),
        }],
    };

    let badge_count = user.global_unread_messages_count().await;

    NotificationBatch {
        badge_count,
        removals: Vec::new(),
        additions: notifications,
    }
}
