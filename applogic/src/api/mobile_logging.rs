// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Once, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use lazy_static::lazy_static;
use log::{error, info, warn, Level, Log, Metadata, Record};
use simplelog::{
    ColorChoice, CombinedLogger, Config, ConfigBuilder, SharedLogger, TermLogger, TerminalMode,
};

use crate::StreamSink;

pub use log::LevelFilter;

static INIT_LOGGER_ONCE: Once = Once::new();

pub fn create_log_stream(s: StreamSink<LogEntry>) -> Result<()> {
    SendToDartLogger::set_stream_sink(s);
    Ok(())
}

pub struct LogEntry {
    pub time_millis: i64,
    pub level: i32,
    pub tag: String,
    pub msg: String,
}

pub fn init_logger() {
    // https://stackoverflow.com/questions/30177845/how-to-initialize-the-logger-for-integration-tests
    INIT_LOGGER_ONCE.call_once(|| {
        let level = if cfg!(debug_assertions) {
            LevelFilter::Info
        } else {
            LevelFilter::Warn
        };

        assert!(
            level <= log::STATIC_MAX_LEVEL,
            "Should respect log::STATIC_MAX_LEVEL={:?}, which is done in compile time. level{:?}",
            log::STATIC_MAX_LEVEL,
            level
        );

        CombinedLogger::init(vec![
            Box::new(SendToDartLogger::new(level)),
            // #[cfg(not(any(target_os = "android", target_os = "ios")))]
            TermLogger::new(
                level,
                ConfigBuilder::new().build(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
        ])
        .unwrap_or_else(|e| {
            error!("init_logger (inside 'once') has error: {:?}", e);
        });
        info!("init_logger (inside 'once') finished");

        warn!(
            "init_logger finished, chosen level={:?} (deliberately output by warn level)",
            level
        );
    });
}

lazy_static! {
    static ref SEND_TO_DART_LOGGER_STREAM_SINK: RwLock<Option<StreamSink<LogEntry>>> =
        RwLock::new(None);
}

pub struct SendToDartLogger {
    level: LevelFilter,
}

impl SendToDartLogger {
    pub fn set_stream_sink(stream_sink: StreamSink<LogEntry>) {
        let prev_stream_sink = {
            let mut guard = SEND_TO_DART_LOGGER_STREAM_SINK.write().expect("poisoned");
            // Note: previous stream sink MUST NOT be dropped before the `guard` is released.
            // On drop, it will log and because the `RwLock` is not reentrant, this will deadlock.
            guard.replace(stream_sink)
        };
        if prev_stream_sink.is_some() {
            warn!(
                "SendToDartLogger::set_stream_sink but already exist a sink, thus overriding. \
                (This may or may not be a problem. It will happen normally if hot-reload Flutter app.)"
            );
        }
    }

    pub fn new(level: LevelFilter) -> Self {
        SendToDartLogger { level }
    }

    fn record_to_entry(record: &Record) -> LogEntry {
        let time_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_millis() as i64;

        let level = match record.level() {
            Level::Trace => Self::LEVEL_TRACE,
            Level::Debug => Self::LEVEL_DEBUG,
            Level::Info => Self::LEVEL_INFO,
            Level::Warn => Self::LEVEL_WARN,
            Level::Error => Self::LEVEL_ERROR,
        };

        let tag = record.file().unwrap_or_else(|| record.target()).to_owned();

        let msg = format!("{}", record.args());

        LogEntry {
            time_millis,
            level,
            tag,
            msg,
        }
    }

    const LEVEL_TRACE: i32 = 5000;
    const LEVEL_DEBUG: i32 = 10000;
    const LEVEL_INFO: i32 = 20000;
    const LEVEL_WARN: i32 = 30000;
    const LEVEL_ERROR: i32 = 40000;
}

impl Log for SendToDartLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let entry = Self::record_to_entry(record);
        if let Some(sink) = &*SEND_TO_DART_LOGGER_STREAM_SINK.read().unwrap() {
            let _ = sink.add(entry);
        }
    }

    fn flush(&self) {
        // no need
    }
}

impl SharedLogger for SendToDartLogger {
    fn level(&self) -> LevelFilter {
        self.level
    }

    fn config(&self) -> Option<&Config> {
        None
    }

    fn as_log(self: Box<Self>) -> Box<dyn Log> {
        Box::new(*self)
    }
}
