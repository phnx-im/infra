// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    panic::{self, AssertUnwindSafe},
};
use tokio::runtime::Builder;

use crate::api::{mobile_logging::init_logger, user::User};

#[derive(Serialize, Deserialize)]
struct IncomingNotificationContent {
    title: String,
    body: String,
    data: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct NotificationBatch {
    badge_count: u32,
    removals: Vec<String>,
    additions: Vec<NotificationContent>,
}

#[derive(Serialize, Deserialize)]
struct NotificationContent {
    identifier: String,
    title: String,
    body: String,
    data: String,
}

#[no_mangle]
pub extern "C" fn process_new_messages(content: *const c_char) -> *mut c_char {
    let c_str = unsafe {
        assert!(!content.is_null());
        CStr::from_ptr(content)
    };

    init_logger();

    let json_str = c_str.to_str().unwrap();
    let incoming_content: IncomingNotificationContent = serde_json::from_str(json_str).unwrap();

    // Retrieve messages
    let batch = retrieve_messages_sync(incoming_content.path);

    let response = serde_json::to_string(&batch).unwrap_or_default();
    CString::new(response).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

fn error_batch(e: String) -> NotificationBatch {
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
fn retrieve_messages_sync(path: String) -> NotificationBatch {
    match Builder::new_multi_thread()
        .thread_name("nse-thread")
        .enable_all()
        .build()
    {
        Ok(runtime) => {
            match panic::catch_unwind(AssertUnwindSafe(|| {
                runtime.block_on(async { retrieve_messages(path).await })
            })) {
                Ok(batch) => batch,
                Err(_) => match panic::catch_unwind(AssertUnwindSafe(|| drop(runtime))) {
                    Ok(_) => {
                        let e = "Failed to execute block_on";
                        log::error!("{}", e);
                        error_batch(e.to_string())
                    }
                    Err(_) => {
                        let e = "Failed to drop runtime";
                        log::error!("{}", e);
                        error_batch(e.to_string())
                    }
                },
            }
        }
        Err(e) => {
            log::error!("Failed to initialize tokio runtime: {}", e);
            error_batch(e.to_string())
        }
    }
}

/// Load the user and retrieve messages
async fn retrieve_messages(path: String) -> NotificationBatch {
    log::info!("Retrieving messages with DB path: {}", path);
    let res = User::load_default(path).await;
    if let Err(e) = res {
        log::error!("Failed to load user: {}", e);
        return error_batch(e.to_string());
    }
    let user = res.unwrap();

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

    NotificationBatch {
        badge_count: 0,
        removals: Vec::new(),
        additions: notifications,
    }
}
