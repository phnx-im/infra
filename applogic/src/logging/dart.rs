// SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;
use std::sync::LazyLock;

use chrono::Utc;
use parking_lot::RwLock;
use tracing::field::{Field, Visit};
use tracing::{warn, Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::api::logging::LogEntry;
use crate::StreamSink;

static DART_SINK: LazyLock<RwLock<Option<StreamSink<LogEntry>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Tracing layer which forwards logs to the Dart side
pub(super) fn layer<S>(tag: &'static str) -> impl Layer<S>
where
    S: Subscriber,
    for<'span> S: LookupSpan<'span>,
{
    SendToDartLayer { tag }
}

pub(crate) fn set_stream_sink(stream_sink: StreamSink<LogEntry>) {
    let prev_stream_sink = {
        let mut guard = DART_SINK.write();
        // Note: The previous stream sink MUST NOT be dropped before the `guard` is released.
        // On drop, it will log a message, and therefore lock the sink for reading. Because the
        // `RwLock` is not reentrant, this will deadlock.
        guard.replace(stream_sink)
    };
    if prev_stream_sink.is_some() {
        warn!(
            "SendToDartLogger::set_stream_sink but already exist a sink, thus overriding. \
            (This may or may not be a problem. It will happen normally if hot-reload Flutter app.)"
        );
    }
}

struct SendToDartLayer {
    tag: &'static str,
}

impl SendToDartLayer {
    fn log(&self, level: tracing::Level, target: String, logline: String) {
        let time = Utc::now();
        let entry = LogEntry {
            time,
            level: level.into(),
            tag: self.tag.to_string(),
            target,
            msg: logline,
        };
        if let Some(sink) = DART_SINK.read().as_ref() {
            let _ = sink.add(entry);
        }
    }
}

impl<S> Layer<S> for SendToDartLayer
where
    S: Subscriber + 'static,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        if ctx.enabled(metadata) {
            let level = *metadata.level();
            let mut target = metadata.target().to_owned();
            let mut logline = String::new();
            event.record(&mut Visitor::new(&mut target, &mut logline));
            self.log(level, target, logline);
        }
    }
}

/// Collect the target, and the message and structured values into `logline`
struct Visitor<'a> {
    target: &'a mut String,
    logline: &'a mut String,
}

impl<'a> Visitor<'a> {
    fn new(target: &'a mut String, logline: &'a mut String) -> Self {
        Self { target, logline }
    }
}

impl Visit for Visitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let field_name = field.name();
        if field_name == "message" {
            write!(self.logline, "{value:?}").expect("infallible");
        } else if !field_name.starts_with("log.") {
            // don't include special `log` fields
            write!(self.logline, " {field_name}={value:?}").expect("infallible");
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let field_name = field.name();
        if field_name == "message" {
            write!(self.logline, "{value}").expect("infallible");
        } else if !field_name.starts_with("log.") {
            // don't include special `log` fields
            write!(self.logline, " {field_name}={value}").expect("infallible");
        } else if field_name.starts_with("log.target") {
            write!(self.target, "{value}").expect("infallible");
        }
    }
}
