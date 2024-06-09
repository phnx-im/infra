// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;

// Misc. functions

use crate::StreamSink;

use super::mobile_logging::{init_logger, LogEntry, SendToDartLogger};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    init_logger();
}

pub fn create_log_stream(s: StreamSink<LogEntry>) -> Result<()> {
    SendToDartLogger::set_stream_sink(s);
    Ok(())
}

pub fn delete_databases(client_db_path: String) -> Result<()> {
    phnxcoreclient::delete_databases(client_db_path.as_str())
}
