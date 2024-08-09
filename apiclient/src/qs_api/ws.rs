// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use core::time;
use std::time::Duration;

use futures_util::{pin_mut, SinkExt, StreamExt};
use http::{HeaderValue, Request};
use phnxtypes::{
    endpoint_paths::ENDPOINT_QS_WS,
    identifiers::QsClientId,
    messages::{client_ds::QsWsMessage, client_qs::QsOpenWsParams},
};
use thiserror::*;
use tls_codec::Serialize;
use tokio::{
    net::TcpStream,
    sync::broadcast::{self, Receiver, Sender},
    task::JoinHandle,
    time::{sleep, Instant},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::{ApiClient, Protocol};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum WsEvent {
    ConnectedEvent,
    DisconnectedEvent,
    MessageEvent(QsWsMessage),
}

enum ConnectionStatusError {
    ChannelClosed,
}

/// Helper object that handles connection status changes and sends out WsEvent
/// messages.
struct ConnectionStatus {
    connected: bool,
}

impl ConnectionStatus {
    fn new() -> Self {
        Self { connected: false }
    }

    fn set_connected(&mut self, tx: &Sender<WsEvent>) -> Result<(), ConnectionStatusError> {
        if !self.connected {
            if let Err(err) = tx.send(WsEvent::ConnectedEvent) {
                log::error!("Error sending to channel: {}", err);
                self.connected = false;
                return Err(ConnectionStatusError::ChannelClosed);
            }
            self.connected = true;
        }
        Ok(())
    }

    fn set_disconnected(&mut self, tx: &Sender<WsEvent>) -> Result<(), ConnectionStatusError> {
        if self.connected {
            if let Err(err) = tx.send(WsEvent::DisconnectedEvent) {
                log::error!("Error sending to channel: {}", err);
                return Err(ConnectionStatusError::ChannelClosed);
            }
            self.connected = false;
        }
        Ok(())
    }
}

/// A websocket connection to the QS server. See the
/// [`ApiClient::spawn_websocket`] method for more information.
pub struct QsWebSocket {
    rx: Receiver<WsEvent>,
    tx: Sender<WsEvent>,
    handle: JoinHandle<()>,
}

impl QsWebSocket {
    /// Returns the next [`WsEvent`] event. This will block until an event is
    /// sent or the connection is closed (in which case a final `None` is
    /// returned).
    pub async fn next(&mut self) -> Option<WsEvent> {
        match self.rx.recv().await {
            Ok(message) => Some(message),
            Err(e) => {
                log::error!("Error receiving from channel: {}", e);
                None
            }
        }
    }

    /// Subscribe to the event stream
    pub fn subscribe(&self) -> Receiver<WsEvent> {
        self.tx.subscribe()
    }

    /// Join the websocket connection task. This will block until the task has
    /// completed.
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.handle.await
    }

    /// Abort the websocket connection task. This will close the websocket connection.
    pub fn abort(&mut self) {
        self.handle.abort();
    }

    /// Internal helper function to handle an established websocket connection
    async fn handle_connection(
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        tx: &Sender<WsEvent>,
        timeout: u64,
    ) {
        let mut last_ping = Instant::now();

        // Watchdog to monitor the connection.
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        // Pin the stream
        pin_mut!(ws_stream);

        // Initialize the connection status
        let mut connection_status = ConnectionStatus::new();
        if connection_status.set_connected(tx).is_err() {
            // Close the stream if all subscribers of the watch have been dropped
            let _ = ws_stream.close().await;
            return;
        }

        // Loop while the connection is open
        loop {
            tokio::select! {
                // Check if the connection is still alive
                _ = interval.tick() => {
                    let now = Instant::now();
                    // Check if we have reached the timeout
                    if now.duration_since(last_ping) > Duration::from_secs(timeout) {
                        // Change the status to Disconnected and send an event
                        let _ = ws_stream.close().await;
                        if connection_status.set_disconnected(tx).is_err() {
                            // Close the stream if all subscribers of the watch have been dropped
                            log::info!("Closing the connection because all subscribers are dropped");
                            return;
                        }
                    }
                },
                // Wait for a message
                message = ws_stream.next() => {
                    if let Some(Ok(message)) = message {
                        match message {
                            // We received a binary message
                            Message::Binary(data) => {
                                // Reset the last ping time
                                last_ping = Instant::now();
                                // Change the status to Connected and send an event
                                if connection_status.set_connected(tx).is_err() {
                                    // Close the stream if all subscribers of the watch have been dropped
                                    log::info!("Closing the connection because all subscribers are dropped");
                                    let _ = ws_stream.close().await;
                                    return;
                                }
                                // Try to deserialize the message
                                if let Ok(QsWsMessage::QueueUpdate) =
                                    phnxtypes::codec::from_slice::<QsWsMessage>(&data)
                                {
                                    // We received a new message notification from the QS
                                    // Send the event to the channel
                                    if tx.send(WsEvent::MessageEvent(QsWsMessage::QueueUpdate)).is_err() {
                                        log::info!("Closing the connection because all subscribers are dropped");
                                        // Close the stream if all subscribers of the watch have been dropped
                                        let _ = ws_stream.close().await;
                                        return;
                                    }
                                }
                            },
                            // We received a ping
                            Message::Ping(_) => {
                                // We update the last ping time
                                last_ping = Instant::now();
                                if connection_status.set_connected(tx).is_err() {
                                    // Close the stream if all subscribers of the watch have been dropped
                                    log::info!("Closing the connection because all subscribers are dropped");
                                    let _ = ws_stream.close().await;
                                    return;
                                }
                            }
                            Message::Close(_) => {
                                // Change the status to Disconnected and send an
                                // event
                                let _ = connection_status.set_disconnected(tx);
                                // We close the websocket
                                let _ = ws_stream.close().await;
                                return;
                            }
                            _ => {
                            }
                        }
                    } else {
                        // It seems the connection is closed, send disconnect
                        // event
                        let _ = connection_status.set_disconnected(tx);
                        break;
                    }
                },
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum SpawnWsError {
    #[error("Could not serialize parameters")]
    WrongParameters,
    #[error("Malformed URL supplied")]
    WrongUrl,
}

impl ApiClient {
    /// Establish a new websocket connection to the QS.
    ///
    /// The client listens for new message notifications (indicating that new
    /// messages have been put in the queue on the QS) and also listens to ping
    /// messages that the QS regularly sends. This function return a
    /// [`QsWebSocket`] object that can be used to receive the following
    /// events:
    ///
    ///  - [`WsEvent::NewMessageEvent`]: A new message has been put in the queue
    ///        on the QS
    ///  - [`WsEvent::DisconnectedEvent`]: The client has not received any
    ///        messages from the QS for a while (longer than the `timeout`
    ///        parameter)
    ///  - [`WsEvent::ConnectedEvent`]: The client has recently received
    ///        messages from the QS (less than the `timeout` parameter)
    ///
    /// The events indicating the connection status do not fully correlate with
    /// the status of the websocket connection itself. Instead, they only
    /// indicate whether messages (usually ping messages) have been received
    /// recently. This serves as an indicator about the quality of the network
    /// connection to the server.
    ///
    /// Whenever the websocket connection drops, the client will try to
    /// reconnect after a short delay (specified by the `retry_interval`
    /// parameter). This is transparent to the consumer, and only manifests
    /// itself by a [`WsEvent::DisconnectedEvent`] followed by a
    /// [`WsEvent::ConnectedEvent].
    ///
    /// The connection will be closed if all subscribers of the [`QsWebSocket`]
    /// have been dropped, or when it is manually closed with using the
    /// [`QsWebSocket::abort()`] function.
    ///
    /// # Arguments
    ///  -  `queue_id` - The ID of the queue monitor.
    ///  - `timeout` - The timeout for the connection in seconds.
    ///  - `retry_interval` - The interval between connection attempts in seconds.
    ///
    /// # Returns
    /// A new [`QsWebSocket`] that represents the websocket connection.
    pub async fn spawn_websocket(
        &self,
        queue_id: QsClientId,
        timeout: u64,
        retry_interval: u64,
    ) -> Result<QsWebSocket, SpawnWsError> {
        // Set the request parameter
        let qs_ws_open_params = QsOpenWsParams { queue_id };
        let serialized = qs_ws_open_params
            .tls_serialize_detached()
            .map_err(|_| SpawnWsError::WrongParameters)?;
        // Format the URL
        let address = self.build_url(Protocol::Ws, ENDPOINT_QS_WS);
        // We check if the request builds correctly
        let _ = Request::builder()
            .uri(address.clone())
            .header("QsOpenWsParams", serialized.as_slice())
            .body(())
            .map_err(|_| SpawnWsError::WrongUrl)?;

        // We create a channel to send events to
        let (tx, rx) = broadcast::channel(100);

        // We clone the sender, so that we can subscribe to more receivers
        let tx_clone = tx.clone();

        log::info!("Spawning the websocket connection...");

        // Spawn the connection task
        let handle = tokio::spawn(async move {
            // Connection loop
            #[cfg(test)]
            let mut counter = 0;
            loop {
                // We build the request and set a custom header
                let req = match address.clone().into_client_request() {
                    Ok(mut req) => {
                        req.headers_mut().insert(
                            "QsOpenWsParams",
                            HeaderValue::from_bytes(&serialized).unwrap(),
                        );
                        req
                    }
                    Err(e) => {
                        log::error!("Error building request: {}", e);
                        // We exit the loop, which in turn drops the channel's sender
                        break;
                    }
                };
                // Try to establish a connection
                match connect_async(req).await {
                    // The connection was established
                    Ok((ws_stream, _)) => {
                        log::info!("Connected to QS WebSocket");
                        // Hand over the connection to the handler
                        QsWebSocket::handle_connection(ws_stream, &tx, timeout).await;
                    }
                    // The connection was not established, wait and try again
                    Err(e) => {
                        log::error!("Error connecting to QS WebSocket: {}", e);
                        #[cfg(test)]
                        {
                            counter += 1;
                            if counter > 10 {
                                break;
                            }
                        }
                    }
                }
                log::info!(
                    "The websocket was closed, trying to reconnect in {} seconds...",
                    retry_interval
                );
                sleep(time::Duration::from_secs(retry_interval)).await;
            }
        });

        Ok(QsWebSocket {
            rx,
            tx: tx_clone,
            handle,
        })
    }
}
