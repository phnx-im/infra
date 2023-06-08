use phnxapiclient::{qs_api::ws::WsEvent, ApiClient, TransportEncryption};
use phnxbackend::qs::{QsClientId, WebsocketNotifier};

use super::*;

/// Test the websocket reconnect.
#[actix_rt::test]
#[tracing::instrument(name = "Test WS Reconnect", skip_all)]
async fn test_ws_reconnect() {
    let (address, _ws_dispatch) = &spawn_app().await;

    let client_id = QsClientId::random();

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    tracing::info!("Server started: {}", address);

    // Initialize the client
    let client = ApiClient::initialize(address.to_string(), TransportEncryption::Off)
        .expect("Failed to initialize client");

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
async fn test_ws_sending() {
    let (address, ws_dispatch) = &spawn_app().await;

    let client_id = QsClientId::random();

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    tracing::info!("Server started: {}", address);

    // Initialize the client
    let client = ApiClient::initialize(address.to_string(), TransportEncryption::Off)
        .expect("Failed to initialize client");

    let mut ws = client
        .spawn_websocket(client_id.clone(), timeout, retry_interval)
        .await
        .expect("Failed to execute request");

    // The first event should be a Connected event, because the websocket is
    // now connected
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));

    // Dispatch a NewMessage event
    ws_dispatch
        .notify(&client_id)
        .await
        .expect("Failed to dispatch");

    // We expect to receive the NewMessage event
    assert_eq!(ws.next().await, Some(WsEvent::NewMessageEvent));
}
