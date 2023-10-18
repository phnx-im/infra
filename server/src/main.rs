// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{net::TcpListener, sync::Arc};

use mls_assist::openmls_traits::types::SignatureScheme;
use phnxbackend::qs::Fqdn;
use phnxserver::{
    configurations::*,
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configure logging/trace subscription
    let subscriber = get_subscriber("phnxserver".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Load configuration
    let configuration = get_configuration("server/").expect("Could not load configuration.");

    if configuration.application.host == "" {
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

    let ds_storage_provider = MemoryDsStorage::new(domain.clone());
    let qs_storage_provider = Arc::new(MemStorageProvider::new(domain.clone()));
    let as_storage_provider = MemoryAsStorage::new(domain, SignatureScheme::ED25519).unwrap();
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
