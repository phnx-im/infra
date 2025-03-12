// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, layer::SubscriberExt};

/// Helper function that allows silencing a number of chatty modules when using
/// the info level "trace". This is meant to be used only for tests.
#[cfg(test)]
fn _silence_chatty_modules(env_filter: EnvFilter) -> EnvFilter {
    env_filter
        .add_directive("actix-web=info".parse().expect("error parsing directive"))
        .add_directive(
            "actix-web-actors=info"
                .parse()
                .expect("error parsing directive"),
        )
        .add_directive("actix=info".parse().expect("error parsing directive"))
        .add_directive("tokio=info".parse().expect("error parsing directive"))
        .add_directive(
            "tokio-tungstenite=info"
                .parse()
                .expect("error parsing directive"),
        )
        .add_directive(
            "tracing-actix-web=info"
                .parse()
                .expect("error parsing directive"),
        )
        .add_directive("tungstenite=info".parse().expect("error parsing directive"))
        .add_directive("mio=info".parse().expect("error parsing directive"))
        .add_directive("hyper=info".parse().expect("error parsing directive"))
        .add_directive("want=info".parse().expect("error parsing directive"))
}

/// Build a subscriber for the server's tracing events from multiple layers.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // Default to "info" level logging.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    // Silence a few very chatty modules, such that "trace" actualy becomes useful
    // Write everything to stdout for now.
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    // Let's build the tracing subscriber.
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
