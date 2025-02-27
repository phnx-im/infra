// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxcoreclient::store::Store;
use std::panic::{self, AssertUnwindSafe};
use tokio::runtime::Builder;
use tracing::{error, info};

use crate::api::user::User;

use super::{NotificationBatch, NotificationContent};

/// TODO: Debug code to be removed
pub(crate) fn error_batch(title: String, body: String) -> NotificationBatch {
    NotificationBatch {
        badge_count: 0,
        removals: Vec::new(),
        additions: vec![NotificationContent {
            identifier: "".to_string(),
            title,
            body,
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
        .map_err(|error| {
            error!(%error, "Failed to initialize tokio runtime");
            ("Runtime error".to_string(), error.to_string())
        })
        .and_then(|runtime| {
            panic::catch_unwind(AssertUnwindSafe(|| {
                runtime.block_on(async { retrieve_messages(path).await })
            }))
            .map_err(|payload| {
                if let Some(s) = payload.downcast_ref::<&str>() {
                    error!("Panic in tokio runtime: {}", s);
                    ("Panic in tokio runtime".to_string(), s.to_string())
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    error!("Panic in tokio runtime: {}", s);
                    ("Panic in tokio runtime".to_string(), s.clone())
                } else {
                    error!("Panic in tokio runtime occurred with unknown payload type");
                    ("Panic in tokio runtime".to_string(), "Unknown".to_string())
                }
            })
        });

    match result {
        Ok(batch) => batch,
        Err((title, body)) => error_batch(title, body),
    }
}

/// Load the user and retrieve messages
pub(crate) async fn retrieve_messages(path: String) -> NotificationBatch {
    info!(path, "Retrieving messages with DB path");
    let user = match User::load_default(path).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            error!("User not found");
            return error_batch(
                "User not found".to_string(),
                "The database contained no user data".to_string(),
            );
        }
        Err(error) => {
            error!(%error, "Failed to load user");
            return error_batch("Failed to load user".to_string(), error.to_string());
        }
    };

    // capture store notification in below store calls
    let pending_store_notifications = user.user.subscribe_iter();

    let notifications = match user.fetch_all_messages().await {
        Ok(fetched_messages) => {
            info!("All messages fetched");
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

    for store_notification in pending_store_notifications {
        if let Err(error) = Store::enqueue_notification(&user.user, &store_notification).await {
            error!(%error, "Failed to enqueue store notification");
        }
    }

    NotificationBatch {
        badge_count,
        removals: Vec::new(),
        additions: notifications,
    }
}
