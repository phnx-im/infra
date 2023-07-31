#![allow(dead_code)]

// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
};

pub mod setup;

use mls_assist::openmls_traits::types::SignatureScheme;
use once_cell::sync::Lazy;
use phnxbackend::qs::Fqdn;
use phnxserver::{
    configurations::get_configuration,
    endpoints::qs::ws::DispatchWebsocketNotifier,
    network_provider::MockNetworkProvider,
    run,
    storage_provider::memory::{
        auth_service::{EphemeralAsStorage, MemoryAsStorage},
        ds::MemoryDsStorage,
        qs::MemStorageProvider,
        qs_connector::MemoryEnqueueProvider,
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
/// address and a DispatchWebsocketNotifier to dispatch notofication over the
/// websocket.
pub async fn spawn_app(
    domain: Fqdn,
    network_provider: Arc<MockNetworkProvider>,
    random_port: bool,
) -> (SocketAddr, DispatchWebsocketNotifier) {
    // Initialize tracing subscription only once.
    Lazy::force(&TRACING);

    // Load configuration
    let _configuration = get_configuration("").expect("Could not load configuration.");

    // Port binding
    let localhost = "127.0.0.1";
    let port = if random_port { 0 } else { 8000 };
    let listener =
        TcpListener::bind(format!("{localhost}:{port}")).expect("Failed to bind to random port.");
    let address = listener.local_addr().unwrap();

    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();

    let ds_storage_provider = MemoryDsStorage::new(domain.clone());
    let qs_storage_provider = Arc::new(MemStorageProvider::new(domain.clone()));

    let as_storage_provider =
        MemoryAsStorage::new(domain.clone(), SignatureScheme::ED25519).unwrap();
    let as_ephemeral_storage_provider = EphemeralAsStorage::default();

    let qs_connector = MemoryEnqueueProvider {
        storage: qs_storage_provider.clone(),
        notifier: ws_dispatch_notifier.clone(),
        network: network_provider.clone(),
    };

    // Start the server
    let server = run(
        listener,
        ws_dispatch_notifier.clone(),
        ds_storage_provider,
        qs_storage_provider,
        as_storage_provider,
        as_ephemeral_storage_provider,
        qs_connector,
    )
    .expect("Failed to bind to address.");

    // Execute the server in the background
    tokio::spawn(server);

    // Return the address
    (address, ws_dispatch_notifier)
}
