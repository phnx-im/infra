// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use tracing::error;

use crate::logging::init_logger;

pub mod conversation_details_cubit;
pub mod conversation_list_cubit;
pub mod conversations;
pub mod logging;
pub mod messages;
pub mod notifications;
pub mod types;
pub mod user;
pub mod utils;

#[frb(init)]
pub fn init() {
    init_logger();

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        if let Err(error) = crate::notifier::init_desktop_os_notifications() {
            error!(%error, "Failed to initialize desktop notifications");
        }
    }
}
