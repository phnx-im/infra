// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use core::time;
use std::{pin::pin, time::Duration};

use base64::{Engine as _, engine::general_purpose};
use futures_util::{SinkExt, StreamExt};
use http::{HeaderValue, Request};
use phnxtypes::{
    codec::PhnxCodec,
    endpoint_paths::ENDPOINT_QS_WS,
    identifiers::QsClientId,
    messages::{client_ds::QsWsMessage, client_qs::QsOpenWsParams},
};
use thiserror::*;
use tls_codec::DeserializeBytes;
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time::{Instant, sleep},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{error, info};
use uuid::Uuid;

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

    async fn set_connected(
        &mut self,
        tx: &mpsc::Sender<WsEvent>,
    ) -> Result<(), ConnectionStatusError> {
        if !self.connected {
            if let Err(error) = tx.send(WsEvent::ConnectedEvent).await {
                error!(%error, "Error sending to channel");
                self.connected = false;
                return Err(ConnectionStatusError::ChannelClosed);
            }
            self.connected = true;
        }
        Ok(())
    }

    async fn set_disconnected(
        &mut self,
        tx: &mpsc::Sender<WsEvent>,
    ) -> Result<(), ConnectionStatusError> {
        if self.connected {
            if let Err(error) = tx.send(WsEvent::DisconnectedEvent).await {
                error!(%error, "Error sending to channel");
                return Err(ConnectionStatusError::ChannelClosed);
            }
            self.connected = false;
        }
        Ok(())
    }
}

/// A websocket connection to the QS server. See the
/// [`ApiClient::spawn_websocket`] method for more information.
///
/// When dropped, the websocket connection will be closed.
pub struct QsWebSocket {
    rx: mpsc::Receiver<WsEvent>,
    _cancel: DropGuard,
}

impl QsWebSocket {
    /// Returns the next [`WsEvent`] event. This will block until an event is
    /// sent or the connection is closed (in which case a final `None` is
    /// returned).
    pub async fn next(&mut self) -> Option<WsEvent> {
        self.rx.recv().await
    }

    /// Internal helper function to handle an established websocket connection
    ///
    /// Returns `true` if the connection should be re-established, otherwise `false`.
    async fn handle_connection(
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        tx: &mpsc::Sender<WsEvent>,
        timeout: u64,
        cancel: &CancellationToken,
    ) -> bool {
        let mut last_ping = Instant::now();

        // Watchdog to monitor the connection.
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        // Pin the stream
        let mut ws_stream = pin!(ws_stream);

        // Initialize the connection status
        let mut connection_status = ConnectionStatus::new();
        if connection_status.set_connected(tx).await.is_err() {
            // Close the stream if all subscribers of the watch have been dropped
            let _ = ws_stream.close().await;
            return false;
        }

        // Loop while the connection is open
        loop {
            tokio::select! {
                // Check is the handler is cancelled
                _ = cancel.cancelled() => {
                    info!("QS WebSocket connection cancelled");
                    break false;
                },
                // Check if the connection is still alive
                _ = interval.tick() => {
                    let now = Instant::now();
                    // Check if we have reached the timeout
                    if now.duration_since(last_ping) > Duration::from_secs(timeout) {
                        // Change the status to Disconnected and send an event
                        let _ = ws_stream.close().await;
                        if connection_status.set_disconnected(tx).await.is_err() {
                            // Close the stream if all subscribers of the watch have been dropped
                            info!("Closing the connection because all subscribers are dropped");
                            return false;
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
                                if connection_status.set_connected(tx).await.is_err() {
                                    // Close the stream if all subscribers of the watch have been dropped
                                    info!("Closing the connection because all subscribers are dropped");
                                    let _ = ws_stream.close().await;
                                    return false;
                                }
                                // Try to deserialize the message
                                if let Ok(QsWsMessage::QueueUpdate) =
                                    QsWsMessage::tls_deserialize_exact_bytes(data.as_slice())
                                {
                                    // We received a new message notification from the QS
                                    // Send the event to the channel
                                    if tx.send(WsEvent::MessageEvent(QsWsMessage::QueueUpdate)).await.is_err() {
                                        info!("Closing the connection because all subscribers are dropped");
                                        // Close the stream if all subscribers of the watch have been dropped
                                        let _ = ws_stream.close().await;
                                        return false;
                                    }
                                }
                            },
                            // We received a ping
                            Message::Ping(_) => {
                                // We update the last ping time
                                last_ping = Instant::now();
                                if connection_status.set_connected(tx).await.is_err() {
                                    // Close the stream if all subscribers of the watch have been dropped
                                    info!("Closing the connection because all subscribers are dropped");
                                    let _ = ws_stream.close().await;
                                    return false;
                                }
                            }
                            Message::Close(_) => {
                                // Change the status to Disconnected and send an
                                // event
                                let _ = connection_status.set_disconnected(tx).await;
                                // We close the websocket
                                let _ = ws_stream.close().await;
                                return true;
                            }
                            _ => {
                            }
                        }
                    } else {
                        // It seems the connection is closed, send disconnect
                        // event
                        let _ = connection_status.set_disconnected(tx).await;
                        break true;
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
    ///  - [`WsEvent::MessageEvent`]: A new message has been put in the queue
    ///    on the QS
    ///  - [`WsEvent::DisconnectedEvent`]: The client has not received any
    ///    messages from the QS for a while (longer than the `timeout`
    ///    parameter)
    ///  - [`WsEvent::ConnectedEvent`]: The client has recently received
    ///    messages from the QS (less than the `timeout` parameter)
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
    /// have been dropped, or when it is manually closed by cancelling the token
    /// `cancel`.
    ///
    /// # Arguments
    ///  -  `queue_id` - The ID of the queue monitor.
    ///  - `timeout` - The timeout for the connection in seconds.
    ///  - `retry_interval` - The interval between connection attempts in seconds.
    ///  - `cancel` - The cancellation token to stop the socket.
    ///
    /// # Returns
    /// A new [`QsWebSocket`] that represents the websocket connection.
    pub async fn spawn_websocket(
        &self,
        queue_id: QsClientId,
        timeout: u64,
        retry_interval: u64,
        cancel: CancellationToken,
    ) -> Result<QsWebSocket, SpawnWsError> {
        // Set the request parameter
        let qs_ws_open_params = QsOpenWsParams { queue_id };
        let serialized =
            PhnxCodec::to_vec(&qs_ws_open_params).map_err(|_| SpawnWsError::WrongParameters)?;
        let encoded = general_purpose::STANDARD.encode(&serialized);
        // Format the URL
        let address = self.build_url(Protocol::Ws, ENDPOINT_QS_WS);
        // We check if the request builds correctly
        let _ = Request::builder()
            .uri(address.clone())
            .header("QsOpenWsParams", &encoded)
            .body(())
            .map_err(|error| {
                error!(%error, "Wrong URL");
                SpawnWsError::WrongUrl
            })?;

        // We create a channel to send events to
        let (tx, rx) = mpsc::channel(100);

        let connection_id = Uuid::new_v4();
        info!(%connection_id, "Spawning the websocket connection...");

        let cancel_guard = cancel.clone().drop_guard();

        // Spawn the connection task
        tokio::spawn(async move {
            // Connection loop
            #[cfg(test)]
            let mut counter = 0;
            while !cancel.is_cancelled() {
                // We build the request and set a custom header
                let req = match address.clone().into_client_request() {
                    Ok(mut req) => {
                        req.headers_mut()
                            .insert("QsOpenWsParams", HeaderValue::from_str(&encoded).unwrap());
                        req
                    }
                    Err(error) => {
                        error!(%error, "Error building request");
                        // We exit the loop, which in turn drops the channel's sender
                        break;
                    }
                };
                // Try to establish a connection
                match connect_async(req).await {
                    // The connection was established
                    Ok((ws_stream, _)) => {
                        info!(%connection_id, "Connected to QS WebSocket");
                        // Hand over the connection to the handler
                        if !QsWebSocket::handle_connection(ws_stream, &tx, timeout, &cancel).await {
                            break;
                        }
                    }
                    // The connection was not established, wait and try again
                    Err(error) => {
                        error!(%error, "Error connecting to QS WebSocket");
                        #[cfg(test)]
                        {
                            counter += 1;
                            if counter > 10 {
                                break;
                            }
                        }
                    }
                }
                info!(
                    %connection_id,
                    retry_in_sec = retry_interval,
                    is_cancelled = cancel.is_cancelled(),
                    "The websocket was closed, will reconnect...",
                );
                sleep(time::Duration::from_secs(retry_interval)).await;
            }

            info!(%connection_id, "QS WebSocket closed");
        });

        Ok(QsWebSocket {
            rx,
            _cancel: cancel_guard,
        })
    }
}
