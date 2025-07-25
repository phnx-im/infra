// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use phnxbackend::{
    auth_service::AuthService,
    ds::{Ds, storage::Storage},
    infra_service::InfraService,
    qs::Qs,
};
use phnxcommon::identifiers::Fqdn;
use phnxserver::{
    RateLimitsConfig, ServerRunParams,
    configurations::*,
    dispatch::DispatchNotifier,
    enqueue_provider::SimpleEnqueueProvider,
    network_provider::MockNetworkProvider,
    push_notification_provider::ProductionPushNotificationProvider,
    run,
    telemetry::{get_subscriber, init_subscriber},
};
use tracing::info;

// TODO: start actix rt?
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure logging/trace subscription
    let subscriber = get_subscriber("phnxserver".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Load configuration
    let mut configuration = get_configuration("server/").expect("Could not load configuration.");

    if configuration.application.domain.is_empty() {
        panic!("No domain name configured.");
    }

    // Port binding
    let addr = format!(
        "{}:{}",
        configuration.application.host, configuration.application.grpc_port
    );
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    let domain: Fqdn = configuration
        .application
        .domain
        .parse()
        .expect("Invalid domain");
    info!(%domain, "Starting server");
    let network_provider = MockNetworkProvider::new();

    let base_db_name = configuration.database.name.clone();
    // DS storage provider
    configuration.database.name = format!("{base_db_name}_ds");
    info!(
        host = configuration.database.host,
        "Connecting to postgres server",
    );
    let mut counter = 0;
    let mut ds_result = Ds::new(&configuration.database, domain.clone()).await;

    // Try again for 10 times each second in case the postgres server is coming up.
    while let Err(e) = ds_result {
        info!("Failed to connect to postgres server: {}", e);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        counter += 1;
        if counter > 10 {
            panic!("Database not ready after 10 seconds.");
        }
        ds_result = Ds::new(&configuration.database, domain.clone()).await;
    }
    let mut ds = ds_result.unwrap();
    if let Some(storage_settings) = &configuration.storage {
        let storage = Storage::new(storage_settings.clone());
        ds.set_storage(storage);
    }

    // New database name for the QS provider
    configuration.database.name = format!("{base_db_name}_qs");
    // QS storage provider
    let qs = Qs::new(&configuration.database, domain.clone())
        .await
        .expect("Failed to connect to database.");

    // New database name for the AS provider
    configuration.database.name = format!("{base_db_name}_as");
    let auth_service = AuthService::new(&configuration.database, domain.clone())
        .await
        .expect("Failed to connect to database.");

    let dispatch_notifier = DispatchNotifier::new();
    let push_notification_provider =
        ProductionPushNotificationProvider::new(configuration.fcm, configuration.apns)?;
    let qs_connector = SimpleEnqueueProvider {
        qs: qs.clone(),
        notifier: dispatch_notifier.clone(),
        push_notification_provider,
        network: network_provider.clone(),
    };

    // Start the server
    let server = run(ServerRunParams {
        listener,
        ds,
        auth_service,
        qs,
        qs_connector,
        dispatch_notifier,
        rate_limits: RateLimitsConfig {
            period: Duration::from_millis(500),
            burst_size: 20,
        },
    })
    .await;

    server.await?;
    Ok(())
}
