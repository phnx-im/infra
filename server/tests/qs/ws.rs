// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use opaque_ke::rand::rngs::OsRng;
use phnxapiclient::{ApiClient, qs_api::ws::WsEvent};
use phnxbackend::qs::{WebsocketNotifier, WsNotification};
use phnxserver::network_provider::MockNetworkProvider;
use phnxserver_test_harness::utils::spawn_app;
use phnxtypes::{identifiers::QsClientId, messages::client_ds::QsWsMessage};
use tokio_util::sync::CancellationToken;
use tracing::info;

/// Test the websocket reconnect.
#[actix_rt::test]
#[tracing::instrument(name = "Test WS Reconnect", skip_all)]
async fn ws_reconnect() {
    let network_provider = MockNetworkProvider::new();
    let ((http_addr, grpc_addr), _ws_dispatch) =
        spawn_app(Some("example.com".parse().unwrap()), network_provider).await;

    let client_id = QsClientId::random(&mut OsRng);

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    info!(%http_addr, %grpc_addr, "Server started");

    // Initialize the client
    let address = format!("http://{http_addr}");
    let client = ApiClient::with_default_http_client(address).expect("Failed to initialize client");

    let cancel = CancellationToken::new();
    let mut ws = client
        .spawn_websocket(client_id, timeout, retry_interval, cancel)
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
    let ((http_addr, grpc_addr), ws_dispatch) =
        spawn_app(Some("example.com".parse().unwrap()), network_provider).await;

    let client_id = QsClientId::random(&mut OsRng);

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    info!(%http_addr, %grpc_addr, "Server started");

    // Initialize the client
    let address = format!("http://{http_addr}");
    let client = ApiClient::with_default_http_client(address).expect("Failed to initialize client");

    let cancel = CancellationToken::new();
    let mut ws = client
        .spawn_websocket(client_id, timeout, retry_interval, cancel)
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
