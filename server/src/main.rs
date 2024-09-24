// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{net::TcpListener, sync::Arc};

use phnxbackend::{auth_service::AuthService, ds::Ds, persistence::InfraService, qs::Qs};
use phnxserver::{
    configurations::*,
    endpoints::qs::{
        push_notification_provider::ProductionPushNotificationProvider,
        ws::DispatchWebsocketNotifier,
    },
    network_provider::MockNetworkProvider,
    run,
    storage_provider::{
        memory::qs_connector::MemoryEnqueueProvider, postgres::qs::PostgresQsStorage,
    },
    telemetry::{get_subscriber, init_subscriber},
};
use phnxtypes::identifiers::Fqdn;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configure logging/trace subscription
    let subscriber = get_subscriber("phnxserver".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Load configuration
    let mut configuration = get_configuration("server/").expect("Could not load configuration.");

    if configuration.application.domain.is_empty() {
        panic!("No domain name configured.");
    }

    // Port binding
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind to random port.");
    let domain: Fqdn = configuration
        .application
        .domain
        .try_into()
        .expect("Invalid domain.");
    tracing::info!("Starting server with domain {}.", domain);
    let network_provider = MockNetworkProvider::new();

    let base_db_name = configuration.database.name.clone();
    // DS storage provider
    configuration.database.name = format!("{}_ds", base_db_name);
    tracing::info!(
        "Connecting to postgres server at {}.",
        configuration.database.host
    );
    let mut counter = 0;
    let mut ds_result = Ds::new(
        &configuration.database.connection_string_without_database(),
        &configuration.database.name,
        domain.clone(),
    )
    .await;

    // Try again for 10 times each second in case the postgres server is coming up.
    while let Err(e) = ds_result {
        tracing::info!("Failed to connect to postgres server: {}", e);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        counter += 1;
        if counter > 10 {
            panic!("Database not ready after 10 seconds.");
        }
        ds_result = Ds::new(
            &configuration.database.connection_string_without_database(),
            &configuration.database.name,
            domain.clone(),
        )
        .await;
    }
    let ds = ds_result.unwrap();

    // New database name for the QS provider
    configuration.database.name = format!("{}_qs", base_db_name);
    // QS storage provider
    let qs = Qs::new(
        &configuration.database.connection_string_without_database(),
        &configuration.database.name,
        domain.clone(),
    )
    .await
    .expect("Failed to connect to database.");
    let qs_storage_provider = Arc::new(
        PostgresQsStorage::new(&configuration.database, domain.clone())
            .await
            .expect("Failed to connect to database."),
    );

    // New database name for the AS provider
    configuration.database.name = format!("{}_as", base_db_name);
    let auth_service = AuthService::new(
        &configuration.database.connection_string_without_database(),
        &configuration.database.name,
        domain.clone(),
    )
    .await
    .expect("Failed to connect to database.");
    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();
    let push_notification_provider = ProductionPushNotificationProvider::new(configuration.apns)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let qs_connector = MemoryEnqueueProvider {
        storage: qs_storage_provider.clone(),
        notifier: ws_dispatch_notifier.clone(),
        push_notification_provider,
        network: network_provider.clone(),
    };
    // Start the server
    run(
        listener,
        ds,
        auth_service,
        qs,
        qs_storage_provider,
        qs_connector,
        network_provider,
        ws_dispatch_notifier,
    )?
    .await
}
