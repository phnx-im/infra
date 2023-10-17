// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use phnxapiclient::{qs_api::ws::WsEvent, ApiClient};
use phnxbackend::qs::{WebsocketNotifier, WsNotification};
use phnxserver::network_provider::MockNetworkProvider;
use phnxtypes::{identifiers::QsClientId, messages::client_ds::QsWsMessage};

use super::*;

/// Test the websocket reconnect.
#[actix_rt::test]
#[tracing::instrument(name = "Test WS Reconnect", skip_all)]
async fn ws_reconnect() {
    let network_provider = MockNetworkProvider::new();
    let (address, _ws_dispatch) = spawn_app("example.com".into(), network_provider, true).await;

    let client_id = QsClientId::random();

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    tracing::info!("Server started: {}", address.to_string());

    // Initialize the client
    let address = format!("http://{}", address);
    let client = ApiClient::initialize(address).expect("Failed to initialize client");

    let mut ws = client
        .spawn_websocket(client_id, timeout, retry_interval)
        .await
        .expect("Failed to execute request");

    // The first event should be a Connected event, because the websocket is
    // now connected
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));

    // The second event should be a Disconnected event, because the timeout was
    // chosen so that it is much shorter than the ping interval
    assert_eq!(ws.next().await, Some(WsEvent::DisconnectedEvent));

    // The third event should be a Connected event again, because we received a
    // ping in the meantime
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));
}

/// Test the websocket sending.
#[actix_rt::test]
#[tracing::instrument(name = "Test WS Sending", skip_all)]
async fn ws_sending() {
    let network_provider = MockNetworkProvider::new();
    let (address, ws_dispatch) = spawn_app("example.com".into(), network_provider, true).await;

    let client_id = QsClientId::random();

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    tracing::info!("Server started: {}", address.to_string());

    // Initialize the client
    let address = format!("http://{}", address);
    let client = ApiClient::initialize(address).expect("Failed to initialize client");

    let mut ws = client
        .spawn_websocket(client_id.clone(), timeout, retry_interval)
        .await
        .expect("Failed to execute request");

    // The first event should be a Connected event, because the websocket is
    // now connected
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));

    // Dispatch a NewMessage event
    ws_dispatch
        .notify(&client_id, WsNotification::QueueUpdate)
        .await
        .expect("Failed to dispatch");

    // We expect to receive the NewMessage event
    assert_eq!(
        ws.next().await,
        Some(WsEvent::MessageEvent(QsWsMessage::QueueUpdate))
    );
}
