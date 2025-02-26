// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Facilities for sending logs to the Dart side

use std::{
    io::{self, BufRead, Write},
    path::Path,
    sync::{Arc, LazyLock},
};

use anyhow::bail;
use bytes::Buf;
use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
use regex::Regex;

use crate::{
    StreamSink,
    logging::{LOG_FILE_RING_BUFFER, LOG_FILE_RING_BUFFER_SIZE, init_logger},
    util::{FileRingBuffer, FileRingBufferLock},
};

/// Initializes the Rust logging system
///
/// The logs are sent to Flutter on Android and iOS, and are written to standard error output on
/// Linux/macOS/Windows. The logs are also written to a file specified by the provided `file_path`.
/// The file has a fixed size and is used as a ring buffer.
///
/// The returned [`LogWriter`] can be used to write logs to the file from the Flutter side.
#[frb(sync)]
pub fn init_rust_logging(log_file: String) -> LogWriter {
    let buffer = init_logger(log_file);
    LogWriter { buffer }
}

/// Reads the application logs from the file currently used for writing logs (if any).
pub fn read_app_logs() -> anyhow::Result<String> {
    let Some(buffer) = LOG_FILE_RING_BUFFER.get() else {
        bail!("No application buffer found");
    };
    read_logs_from_buffer(&buffer.lock())
}

/// Clears all pplication logs.
///
/// The file is truncated to zero length, but is is kept open.
pub fn clear_app_logs() -> anyhow::Result<()> {
    let Some(buffer) = LOG_FILE_RING_BUFFER.get() else {
        bail!("No application buffer found");
    };
    buffer.lock().clear();
    Ok(())
}

/// Reads the background logs from the file: `<cache_dir>/background.log`.
pub fn read_background_logs(cache_dir: String) -> anyhow::Result<String> {
    let buffer = open_background_logs_file(cache_dir)?;
    read_logs_from_buffer(&buffer)
}

/// Clears the background logs at `<cache_dir>/background.log`.
///
/// The file is truncated to zero length, but is not deleted.
pub fn clear_background_logs(cache_dir: String) -> anyhow::Result<()> {
    open_background_logs_file(cache_dir)?.clear();
    Ok(())
}

fn open_background_logs_file(cache_dir: String) -> anyhow::Result<FileRingBuffer> {
    let log_file_path = Path::new(&cache_dir).join("background.log");
    Ok(FileRingBuffer::open(
        log_file_path,
        LOG_FILE_RING_BUFFER_SIZE,
    )?)
}

// Note: this function is not memory-allocations optimized.
fn read_logs_from_buffer(buffer: &FileRingBuffer) -> anyhow::Result<String> {
    static ANSI_ESCAPE_SEQUENCE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap());
    let mut lines = buffer
        .buf()
        .reader()
        .split(b'\n')
        .filter_map(|line| {
            let line = line.ok()?;
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_matches('\0');
            let line = ANSI_ESCAPE_SEQUENCE_RE.replace_all(line, "").into_owned();
            Some(line)
        })
        .collect::<Vec<_>>();
    lines.reverse();
    Ok(lines.join("\n"))
}

/// Writes the given log entry to the file initialized by [`init_rust_logging`].
#[frb(opaque)]
pub struct LogWriter {
    buffer: Arc<FileRingBufferLock>,
}

impl LogWriter {
    /// Writes the given log entry with a newline.
    #[frb]
    pub fn write_line(&self, message: &str) -> io::Result<()> {
        let mut buffer = self.buffer.lock();
        writeln!(buffer, "{message}")?;
        Ok(())
    }
}

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
