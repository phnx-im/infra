use std::net::TcpListener;

use phnxserver::{
    configurations::*,
    run,
    storage_provider::memory::{
        ds::MemoryDsStorage, enqueue_provider::MemoryEnqueueProvider, qs::MemStorageProvider,
    },
    telemetry::{get_subscriber, init_subscriber},
};

#[cfg(features = "postgresql")]
use phnxserver::storage_provider::psql::ds::PgDsStorage;

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
    let qs_storage_provider = MemStorageProvider::default();
    // TODO: Here we need to link the enqueue provider and the storage
    // provider.
    // At least for the enqueue provider, we need interior mutability
    // and we also need to ensure that storage and enqueue provider can
    // safely access the same state.
    // The enqueue provider also needs a WS dispatch.
    let qs_enqueue_provider: MemoryEnqueueProvider = todo!();

    // Start the server
    run(
        listener,
        ds_storage_provider,
        qs_storage_provider,
        qs_enqueue_provider,
    )?
    .await
}
