#![allow(dead_code)]
use std::{net::TcpListener, sync::Arc};

use once_cell::sync::Lazy;
use phnxserver::{
    configurations::{get_configuration, Environment},
    endpoints::qs::ws::DispatchWebsocketNotifier,
    run,
    storage_provider::memory::{
        ds::MemoryDsStorage, enqueue_provider::MemoryEnqueueProvider, qs::MemStorageProvider,
    },
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // This allows us to choose not to capture traces for tests that pass.
    // To get all logs just run `TEST_LOG=true cargo test health_check_works | bunyan`.
    // bunyan can be installed via `cargo install bunyan`.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

/// Start the server and initialize the database connection. Returns the
/// address.
pub async fn spawn_app() -> (String, DispatchWebsocketNotifier) {
    // Initialize tracing subscription only once.
    Lazy::force(&TRACING);

    // Load configuration
    let _configuration = get_configuration("").expect("Could not load configuration.");

    // Port binding
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port.");
    let port = listener.local_addr().unwrap().port();

    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();

    match Environment::from_env().expect("Invalid value for APP_ENVIRONMENT found") {
        Environment::Local => {
            let ds_storage_provider = MemoryDsStorage::new();
            let qs_storage_provider = Arc::new(MemStorageProvider::default());

            let qs_enqueue_provider = MemoryEnqueueProvider {
                storage: qs_storage_provider.clone(),
                notifier: ws_dispatch_notifier.clone(),
            };

            // Start the server
            let server = run(
                listener,
                ws_dispatch_notifier.clone(),
                ds_storage_provider,
                qs_storage_provider,
                qs_enqueue_provider,
            )
            .expect("Failed to bind to address.");

            // Execute the server in the background
            tokio::spawn(server);
        }
        Environment::Production => {
            let ds_storage_provider = MemoryDsStorage::new();
            let qs_storage_provider = Arc::new(MemStorageProvider::default());

            let qs_enqueue_provider = MemoryEnqueueProvider {
                storage: qs_storage_provider.clone(),
                notifier: ws_dispatch_notifier.clone(),
            };

            // Start the server
            let server = run(
                listener,
                ws_dispatch_notifier.clone(),
                ds_storage_provider,
                qs_storage_provider,
                qs_enqueue_provider,
            )
            .expect("Failed to bind to address.");

            // Execute the server in the background
            tokio::spawn(server);
        }
    };

    // Return the address
    (format!("127.0.0.1:{port}"), ws_dispatch_notifier)
}
