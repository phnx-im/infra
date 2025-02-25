// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Facilities for sending logs to the Dart side

use std::{
    io::{self, Read, Write},
    path::Path,
    sync::{Arc, LazyLock},
};

use anyhow::{bail, Context};
use bytes::Buf;
use chrono::{DateTime, Utc};
use flate2::{write::GzEncoder, Compression};
use flutter_rust_bridge::frb;
use regex::Regex;

use crate::{
    logging::{init_logger, LOG_FILE_RING_BUFFER, LOG_FILE_RING_BUFFER_SIZE},
    util::{FileRingBuffer, FileRingBufferLock},
    StreamSink,
};

/// Initializes the Rust logging system
///
/// The logs are sent to Flutter on Android and iOS, and are writter to standard error output on
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

/// Creates a Zlib compressed tar archive of the logs
pub fn tar_logs(cache_dir: String) -> anyhow::Result<Vec<u8>> {
    let mut data = Vec::with_capacity(2 * LOG_FILE_RING_BUFFER_SIZE);
    let enc = GzEncoder::new(&mut data, Compression::default());
    let mut tar = tar::Builder::new(enc);

    // app logs
    {
        let app_buffer = LOG_FILE_RING_BUFFER
            .get()
            .context("No application buffer found")?;
        let buffer = app_buffer.lock();
        let mut header = tar::Header::new_gnu();
        header.set_size(buffer.len().try_into().expect("overflow"));
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "logs/app.log", buffer.buf().reader())?;
    }

    // background logs
    {
        let buffer = open_background_logs_file(cache_dir)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(buffer.len().try_into().expect("overflow"));
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "logs/background.log", buffer.buf().reader())?;
    }

    tar.finish()?;
    drop(tar);

    Ok(data)
}

fn open_background_logs_file(cache_dir: String) -> anyhow::Result<FileRingBuffer> {
    let log_file_path = Path::new(&cache_dir).join("background.log");
    Ok(FileRingBuffer::open(
        log_file_path,
        LOG_FILE_RING_BUFFER_SIZE,
    )?)
}

fn read_logs_from_buffer(buffer: &FileRingBuffer) -> anyhow::Result<String> {
    static ANSI_ESCAPE_SEQUENCE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap());
    let mut content = String::new();
    buffer.buf().reader().read_to_string(&mut content)?;
    let content = content.lines().rev().collect::<Vec<_>>().join("\n");
    Ok(ANSI_ESCAPE_SEQUENCE_RE
        .replace_all(&content, "")
        .into_owned())
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
        buffer.write_all(message.as_bytes())?;
        buffer.write_all(b"\n")?;
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
