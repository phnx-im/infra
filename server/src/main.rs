// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::TcpListener;

use phnxbackend::{auth_service::AuthService, ds::Ds, infra_service::InfraService, qs::Qs};
use phnxserver::{
    configurations::*,
    endpoints::qs::{
        push_notification_provider::ProductionPushNotificationProvider,
        ws::DispatchWebsocketNotifier,
    },
    enqueue_provider::SimpleEnqueueProvider,
    network_provider::MockNetworkProvider,
    run,
    telemetry::{get_subscriber, init_subscriber},
};
use phnxtypes::identifiers::Fqdn;
use tracing::info;

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
    let ds = ds_result.unwrap();

    // New database name for the QS provider
    configuration.database.name = format!("{}_qs", base_db_name);
    // QS storage provider
    let qs = Qs::new(&configuration.database, domain.clone())
        .await
        .expect("Failed to connect to database.");

    // New database name for the AS provider
    configuration.database.name = format!("{}_as", base_db_name);
    let auth_service = AuthService::new(&configuration.database, domain.clone())
        .await
        .expect("Failed to connect to database.");

    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();
    let push_notification_provider =
        ProductionPushNotificationProvider::new(configuration.fcm, configuration.apns)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    let qs_connector = SimpleEnqueueProvider {
        qs: qs.clone(),
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
        qs_connector,
        network_provider,
        ws_dispatch_notifier,
    )?
    .await
}
