// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use chrono::{DateTime, Utc};

use crate::StreamSink;

pub struct LogEntry {
    pub time: DateTime<Utc>,
    pub level: LogEntryLevel,
    pub target: String,
    pub msg: String,
}

pub enum LogEntryLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<tracing::Level> for LogEntryLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => LogEntryLevel::Trace,
            tracing::Level::DEBUG => LogEntryLevel::Debug,
            tracing::Level::INFO => LogEntryLevel::Info,
            tracing::Level::WARN => LogEntryLevel::Warn,
            tracing::Level::ERROR => LogEntryLevel::Error,
        }
    }
}

pub fn create_log_stream(_s: StreamSink<LogEntry>) {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    crate::logging::dart::set_stream_sink(_s)
}
