// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;

use mobile_logging::{init_logger, LogEntry, SendToDartLogger};
use types::{UiConversation, UiNotificationType};
use user::{User, UserBuilder, WsNotification};

use crate::StreamSink;

pub mod app_state;
pub mod conversations;
pub mod messages;
pub mod mobile_logging;
pub mod notifications;
pub mod types;
pub mod user;

/// This is only to tell flutter_rust_bridge that it should expose the types
/// used by SinkStream
pub fn _e1(x: LogEntry) -> LogEntry {
    x
}
pub fn _e2(x: UiConversation) -> UiConversation {
    x
}
pub fn _e3(x: UiNotificationType) -> UiNotificationType {
    x
}
pub fn _e4(x: WsNotification) -> WsNotification {
    x
}
pub fn _e5(x: User) -> User {
    x
}
pub fn _e6(x: UserBuilder) -> UserBuilder {
    x
}

// Misc. functions

pub fn rust_set_up() {
    init_logger();
}

pub fn create_log_stream(s: StreamSink<LogEntry>) -> Result<()> {
    SendToDartLogger::set_stream_sink(s);
    Ok(())
}

pub fn delete_databases(client_db_path: String) -> Result<()> {
    phnxcoreclient::delete_databases(client_db_path.as_str())?;
    Ok(())
}
