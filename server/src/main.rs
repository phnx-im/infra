// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{net::TcpListener, sync::Arc};

use mls_assist::openmls_traits::types::SignatureScheme;
use phnxserver::{
    configurations::*,
    endpoints::qs::ws::DispatchWebsocketNotifier,
    network_provider::MockNetworkProvider,
    run,
    storage_provider::{
        memory::{auth_service::EphemeralAsStorage, qs_connector::MemoryEnqueueProvider},
        postgres::{auth_service::PostgresAsStorage, ds::PostgresDsStorage, qs::PostgresQsStorage},
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

    if configuration.application.domain == "" {
        panic!("No domain name configured.");
    }

    // Port binding
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind to random port.");
    let domain: Fqdn = configuration.application.domain.into();
    tracing::info!("Starting server with domain {}.", domain);
    let network_provider = MockNetworkProvider::new();

    let base_db_name = configuration.database.database_name.clone();
    // DS storage provider
    configuration.database.database_name = format!("{}_ds", base_db_name);
    tracing::info!("Connecting to DS database");
    let mut counter = 0;
    let mut ds_provider_result =
        PostgresDsStorage::new(&configuration.database, domain.clone()).await;
    // Try again for 10 times each second in case the postgres server is coming up.
    while let Err(e) = ds_provider_result {
        tracing::info!("Waiting for database to be ready: {:?}", e);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        counter += 1;
        if counter > 10 {
            panic!("Database not ready after 10 seconds.");
        }
        ds_provider_result = PostgresDsStorage::new(&configuration.database, domain.clone()).await;
    }
    let ds_storage_provider = ds_provider_result.unwrap();

    // New database name for the QS provider
    configuration.database.database_name = format!("{}_qs", base_db_name);
    // QS storage provider
    tracing::info!("Connecting to QS database");
    let qs_storage_provider = Arc::new(
        PostgresQsStorage::new(&configuration.database, domain.clone())
            .await
            .expect("Failed to connect to database."),
    );

    // New database name for the AS provider
    configuration.database.database_name = format!("{}_as", base_db_name);
    tracing::info!("Connecting to AS database");
    let as_storage_provider = PostgresAsStorage::new(
        domain.clone(),
        SignatureScheme::ED25519,
        &configuration.database,
    )
    .await
    .expect("Failed to connect to database.");
    let as_ephemeral_storage_provider = EphemeralAsStorage::default();
    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();
    let qs_connector = MemoryEnqueueProvider {
        storage: qs_storage_provider.clone(),
        notifier: ws_dispatch_notifier.clone(),
        network: network_provider.clone(),
    };

    // Start the server
    run(
        listener,
        ws_dispatch_notifier,
        ds_storage_provider,
        qs_storage_provider,
        as_storage_provider,
        as_ephemeral_storage_provider,
        qs_connector,
        network_provider,
    )?
    .await
}
