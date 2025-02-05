// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{net::TcpListener, time::Duration};

use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{
    dev::Server,
    middleware::Logger,
    web::{self},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;
use base64::{engine::general_purpose, Engine as _};
use phnxtypes::{
    codec::PhnxCodec,
    endpoint_paths::ENDPOINT_QS_WS,
    identifiers::QsClientId,
    messages::{client_ds::QsWsMessage, client_qs::QsOpenWsParams},
};
use tls_codec::Serialize;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use uuid::Uuid;

use crate::{qs_api::ws::WsEvent, ApiClient};

static QUEUE_ID_VALUE: Uuid = Uuid::nil();

#[tokio::test]
async fn ws_lifecycle() {
    let _ = tracing_subscriber::fmt::try_init();
    // Ask for a random port and create a listener
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port.");
    let address = listener.local_addr().expect("Failed to get local address.");
    let server = run_server(listener).expect("Could not initialize server.");

    // Execute the server in the background
    tokio::spawn(server);

    let queue_id = QsClientId::from(QUEUE_ID_VALUE);

    // Websocket parameters
    let timeout = 1;
    let retry_interval = 1;

    // Initialize the client
    let address = format!("http://{}", address);
    let client = ApiClient::with_default_http_client(address).expect("Failed to initialize client");

    // Spawn the websocket connection task
    let cancel = CancellationToken::new();
    let mut ws = client
        .spawn_websocket(queue_id, timeout, retry_interval, cancel)
        .await
        .expect("Failed to execute request");

    // Initial Connected event
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));
    // Disconnected event because the timeout logic was triggered
    assert_eq!(ws.next().await, Some(WsEvent::DisconnectedEvent));
    // Connected event because we received a ping in the meantime
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));
    // Disconnected event because the timeout logic was triggered
    assert_eq!(ws.next().await, Some(WsEvent::DisconnectedEvent));
    // Connected event because we received a NewMessage event
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));
    // Actual NewMessage event
    assert_eq!(
        ws.next().await,
        Some(WsEvent::MessageEvent(QsWsMessage::QueueUpdate))
    );
    // Disconnected event because the websocket was close from the server side
    assert_eq!(ws.next().await, Some(WsEvent::DisconnectedEvent));
    // Connected event because the client tried to reconnect to the websocket
    assert_eq!(ws.next().await, Some(WsEvent::ConnectedEvent));
}

// === Websocket server ===

fn run_server(listener: TcpListener) -> Result<Server, std::io::Error> {
    let addr = listener.local_addr().unwrap();
    info!(%addr, "Starting server at address",);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route(ENDPOINT_QS_WS, web::get().to(upgrade_connection))
    })
    .listen(listener)?
    .run();
    Ok(server)
}

pub(crate) async fn upgrade_connection(req: HttpRequest, stream: web::Payload) -> impl Responder {
    // Read parameter from the request
    let header_value = match req.headers().get("QsOpenWsParams") {
        Some(value) => value,
        None => {
            error!("No QsOpenWsParams header found");
            return HttpResponse::BadRequest().body("No QsOpenWsParams header");
        }
    };

    // Decode the header value using base64
    let qs_open_ws_params_bytes = match general_purpose::STANDARD.decode(header_value.as_bytes()) {
        Ok(bytes) => bytes,
        Err(error) => {
            error!(%error, "Could not base64-decode QsOpenWsParams header");
            return HttpResponse::BadRequest().body(format!(
                "Could not decode base64 QsOpenWsParams header: {error}",
            ));
        }
    };

    // Deserialize the header value
    let qs_open_ws_params: QsOpenWsParams = match PhnxCodec::from_slice(&qs_open_ws_params_bytes) {
        Ok(value) => value,
        Err(error) => {
            error!(%error, "Could not deserialize QsOpenWsParams header");
            return HttpResponse::BadRequest().body(format!(
                "Could not deserialize QsOpenWsParams header: {error}",
            ));
        }
    };

    // Check the queue id value
    assert_eq!(qs_open_ws_params.queue_id.as_uuid(), &QUEUE_ID_VALUE);

    // Extract the queue ID
    let qs_ws_connection = QsWsConnection::new();

    // Upgrade the connection to a websocket connection
    info!("Upgrading HTTP connection to websocket connection...");
    match ws::start(qs_ws_connection, &req, stream) {
        Ok(res) => res,
        Err(error) => {
            error!(%error, "Error upgrading connection");
            HttpResponse::InternalServerError().body(error.to_string())
        }
    }
}

/// Define the websocket actor. It will handle the websocket connection and
/// lifecycle.
struct QsWsConnection {}

impl QsWsConnection {
    pub(crate) fn new() -> Self {
        QsWsConnection {}
    }
}

impl Actor for QsWsConnection {
    type Context = ws::WebsocketContext<Self>;

    /// This method is called on actor start. We start the heartbeat process
    /// here.
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_later(Duration::from_secs(0), |_act, ctx| {
            // We send a ping
            info!("Sending ping 1");
            ctx.ping(b"Ping 1");
            // We wait for 2 second before we send the next message.
            // This way we make sure to trigger the timeout logic.
            ctx.run_later(Duration::from_secs(2), |_act, ctx| {
                // Then we send a ping again
                info!("Sending ping 2");
                ctx.ping(b"Ping 2");
                // We wait for another 2 second to trigger the timeout logic
                // again
                ctx.run_later(Duration::from_secs(2), |_act, ctx| {
                    // Now we send an actual message
                    // Serialize the message
                    let serialized = QsWsMessage::QueueUpdate
                        .tls_serialize_detached()
                        .expect("Failed to serialize message");
                    // Send the message to the client
                    info!("Sending binary message");
                    ctx.binary(serialized);
                    // Wait for less than a second, so as to not trigger the
                    // timeout logic but still make sure the binary messages
                    // gets delivered
                    ctx.run_later(Duration::from_millis(100), |_act, ctx| {
                        // Finally, we close the websocket from the server side
                        info!("Stopping the context");
                        ctx.stop();
                    });
                });
            });
        });
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for QsWsConnection {
    /// Handle ws::Message message
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws_msg) = msg {
            match ws_msg {
                ws::Message::Continuation(_) => {
                    info!("Continuation message received");
                    ctx.stop();
                }
                ws::Message::Ping(_) => todo!(),
                ws::Message::Pong(bytes) => {
                    info!(?bytes, "Received a pong");
                }
                ws::Message::Close(close_reason) => {
                    info!(?close_reason, "Received a close");
                }
                _ => {
                    info!("Unknown message received");
                }
            };
        }
    }
}
