// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

mod qs;
mod utils;

use phnxapiclient::{ApiClient, TransportEncryption};
pub use utils::*;

#[actix_rt::test]
#[tracing::instrument(name = "Test WS", skip_all)]
async fn health_check_succeeds() {
    tracing::info!("Tracing: Spawning websocket connection task");
    let (address, _ws_dispatch) = &spawn_app().await;

    tracing::info!("Server started: {}", address);
    println!("Server started: {}", address);

    // Initialize the client
    let client = ApiClient::initialize(address.to_string(), TransportEncryption::Off)
        .expect("Failed to initialize client");

    // Do the health check
    assert!(client.health_check().await);
}
