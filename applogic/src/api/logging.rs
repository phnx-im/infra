// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Facilities for sending logs to the Dart side

use chrono::{DateTime, Utc};

use crate::StreamSink;

/// A log entry sent to the Dart side
pub struct LogEntry {
    /// The timestamp of the log entry
    pub time: DateTime<Utc>,
    /// The log level
    pub level: LogEntryLevel,
    /// The target of the log entry (module path)
    pub target: String,
    /// The log message
    ///
    /// Structured data is attached to the end of the message as formatted key-value pairs.
    pub msg: String,
}

/// The log level
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

/// Assigns the given sink as the log sink on the Rust side.
///
/// If there was already a different sink assigned, it is replaced.
///
/// Call this function to forward logs from the Rust side to the Dart side. This is useful to show
/// logs in the Flutter logging output.
///
/// Only done on Android and iOS. On other platforms, logs are printed to standard error output.
pub fn create_log_stream(_s: StreamSink<LogEntry>) {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    crate::logging::dart::set_stream_sink(_s)
}
