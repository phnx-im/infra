// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use flutter_rust_bridge::frb;
use mobile_logging::init_logger;

pub mod conversation_list_cubit;
pub mod conversations;
pub mod messages;
pub mod mobile_logging;
pub mod notifications;
pub mod types;
pub mod user;
pub mod utils;

#[frb(init)]
pub fn init() {
    init_logger();

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        if let Err(e) = crate::notifier::init_desktop_os_notifications() {
            log::error!("Failed to initialize desktop notifications: {}", e);
        }
    }
}
