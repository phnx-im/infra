// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{net::TcpListener, sync::Arc};

use phnxserver::{
    configurations::*,
    endpoints::qs::ws::DispatchWebsocketNotifier,
    run,
    storage_provider::memory::{
        ds::MemoryDsStorage, enqueue_provider::MemoryEnqueueProvider, qs::MemStorageProvider,
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

    // Port binding
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind to random port.");

    let ds_storage_provider = MemoryDsStorage::new();
    let qs_storage_provider = Arc::new(MemStorageProvider::default());
    let ws_dispatch_notifier = DispatchWebsocketNotifier::default_addr();
    let qs_enqueue_provider = MemoryEnqueueProvider {
        storage: qs_storage_provider.clone(),
        notifier: ws_dispatch_notifier.clone(),
    };

    // Start the server
    run(
        listener,
        ws_dispatch_notifier,
        ds_storage_provider,
        qs_storage_provider,
        qs_enqueue_provider,
    )?
    .await
}
