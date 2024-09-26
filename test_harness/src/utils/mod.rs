#![allow(dead_code)]

// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::{SocketAddr, TcpListener};

pub mod setup;

use once_cell::sync::Lazy;
use phnxbackend::{auth_service::AuthService, ds::Ds, persistence::InfraService, qs::Qs};
use phnxserver::{
    configurations::get_configuration,
    endpoints::qs::{
        push_notification_provider::ProductionPushNotificationProvider,
        ws::DispatchWebsocketNotifier,
    },
    network_provider::MockNetworkProvider,
    run,
    storage_provider::memory::qs_connector::MemoryEnqueueProvider,
    telemetry::{get_subscriber, init_subscriber},
};
use phnxtypes::identifiers::Fqdn;
use uuid::Uuid;

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
    domain: impl Into<Option<Fqdn>>,
    network_provider: MockNetworkProvider,
) -> (SocketAddr, DispatchWebsocketNotifier) {
    // Initialize tracing subscription only once.
    Lazy::force(&TRACING);

    // Load configuration
    let mut configuration = get_configuration("../server/").expect("Could not load configuration.");
    configuration.database.name = Uuid::new_v4().to_string();

    // Port binding
    let port = 0;
    let host = configuration.application.host;
    let listener =
        TcpListener::bind(format!("{host}:{port}")).expect("Failed to bind to random port.");
    let domain = domain.into().unwrap_or_else(|| host.try_into().unwrap());
    let address = listener.local_addr().unwrap();

    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();

    // DS storage provider
    let ds = Ds::new(
        &configuration.database.connection_string_without_database(),
        &configuration.database.name,
        domain.clone(),
    )
    .await
    .expect("Failed to connect to database.");

    // New database name for the AS provider
    configuration.database.name = Uuid::new_v4().to_string();

    let auth_service = AuthService::new(
        &configuration.database.connection_string_without_database(),
        &configuration.database.name,
        domain.clone(),
    )
    .await
    .expect("Failed to connect to database.");

    // New database name for the QS provider
    configuration.database.name = Uuid::new_v4().to_string();

    let qs = Qs::new(
        &configuration.database.connection_string_without_database(),
        &configuration.database.name,
        domain.clone(),
    )
    .await
    .expect("Failed to connect to database.");

    let push_notification_provider = ProductionPushNotificationProvider::new(None).unwrap();

    let qs_connector = MemoryEnqueueProvider {
        qs: qs.clone(),
        notifier: ws_dispatch_notifier.clone(),
        push_notification_provider,
        network: network_provider.clone(),
    };

    // Start the server
    let server = run(
        listener,
        ds,
        auth_service,
        qs,
        qs_connector,
        network_provider,
        ws_dispatch_notifier.clone(),
    )
    .expect("Failed to bind to address.");

    // Execute the server in the background
    tokio::spawn(server);

    // Return the address
    (address, ws_dispatch_notifier)
}
