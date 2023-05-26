// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod dispatch;
pub(crate) mod messages;

use actix::{
    clock::Instant, fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext,
    ContextFutureSpawner, Handler, Message, Running, StreamHandler, WrapFuture,
};
use actix_web::{
    web::{self, Data},
    HttpRequest, HttpResponse, Responder,
};
use actix_web_actors::ws::{self};
use async_trait::*;
use dispatch::*;
use messages::*;
use phnxbackend::qs::{QsClientId, WebsocketNotifier, WebsocketNotifierError};
use serde::{Deserialize, Serialize};
use tokio::{self, time::Duration};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize, Deserialize)]
pub struct QsOpenWsParams {
    pub queue_id: QsClientId,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum QsWsMessage {
    NewMessage = 1,
}

pub struct Client {
    pub queue_id: QsClientId,
}

/// Define the websocket actor. It will handle the websocket connection and
/// lifecycle.
struct QsWsConnection {
    queue_id: QsClientId,
    heartbeat: Instant,
    dispatch_addr: Addr<Dispatch>,
}

impl QsWsConnection {
    pub(crate) fn new(queue_id: QsClientId, dispatch_addr: Addr<Dispatch>) -> Self {
        QsWsConnection {
            queue_id,
            heartbeat: Instant::now(),
            dispatch_addr,
        }
    }

    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                log::info!("Disconnecting websocket because heartbeat failed");
                act.dispatch_addr.do_send(Disconnect {
                    queue_id: act.queue_id.clone(),
                });
                ctx.stop();
                return;
            }

            ctx.ping(b"Phoenix");
        });
    }
}

impl Actor for QsWsConnection {
    type Context = ws::WebsocketContext<Self>;

    /// This method is called on actor start. We start the heartbeat process
    /// here.
    fn started(&mut self, ctx: &mut Self::Context) {
        // Start heartbeat task for this connection
        self.heartbeat(ctx);

        // Register the client with dispatch
        let addr = ctx.address();
        self.dispatch_addr
            .send(Connect {
                addr: addr.recipient(),
                own_queue_id: self.queue_id.clone(),
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => (),
                    // If we can't register the client, stop the actor
                    _ => {
                        log::error!("Error registering client with dispatch");
                        ctx.stop()
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    /// This method is called when the actor is dropped.
    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.dispatch_addr.do_send(Disconnect {
            queue_id: self.queue_id.clone(),
        });
        Running::Stop
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for QsWsConnection {
    /// Handle ws::Message message
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws_msg) = msg {
            match ws_msg {
                ws::Message::Continuation(_) => {
                    tracing::trace!("Continuation message received");
                    ctx.stop();
                }
                ws::Message::Ping(_) => todo!(),
                ws::Message::Pong(bytes) => {
                    self.heartbeat = Instant::now();
                    tracing::trace!("Received a pong: {:?}", bytes);
                }
                ws::Message::Close(close_reason) => {
                    tracing::trace!("Received a close: {:?}", close_reason);
                    self.dispatch_addr.do_send(Disconnect {
                        queue_id: self.queue_id.clone(),
                    });
                    ctx.stop()
                }
                _ => {
                    tracing::warn!("Unknown message received");
                }
            };
        }
    }
}

/// Handler for QsWsMessage
impl Handler<QsWsMessage> for QsWsConnection {
    type Result = ();

    fn handle(&mut self, msg: QsWsMessage, ctx: &mut Self::Context) {
        // Serialize the message
        let serialized = serde_json::to_vec(&msg).unwrap();
        // Send the message to the client
        ctx.binary(serialized);
    }
}

/// Upgrade a HTTP connection to a WebSocket connection.
/// TODO: There is no authentication yet.
#[tracing::instrument(
    name = "Upgrade connection to web socket",
    skip(req, stream, dispatch_data)
)]
pub(crate) async fn upgrade_connection(
    req: HttpRequest,
    stream: web::Payload,
    dispatch_data: Data<Addr<Dispatch>>,
) -> impl Responder {
    // Read parameter from the request
    let header_value = match req.headers().get("QsOpenWsParams") {
        Some(value) => value,
        None => {
            tracing::error!("No QsOpenWsParams header found");
            return HttpResponse::BadRequest().body("No QsOpenWsParams header");
        }
    };

    // Decode the header value
    let decoded_header_value = match base64::decode(header_value) {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Could not decode QsOpenWsParams header: {}", e);
            return HttpResponse::BadRequest().body(format!(
                "Could not decode base64 QsOpenWsParams header: {}",
                e
            ));
        }
    };

    // Deserialize the header value
    let qs_open_ws_params: QsOpenWsParams = match serde_json::from_slice(&decoded_header_value) {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Could not deserialize QsOpenWsParams header: {}", e);
            return HttpResponse::BadRequest().body(format!(
                "Could not deserialize QsOpenWsParams header: {}",
                e
            ));
        }
    };

    // Extract the queue ID
    let qs_ws_connection =
        QsWsConnection::new(qs_open_ws_params.queue_id, dispatch_data.get_ref().clone());

    // Upgrade the connection to a websocket connection
    tracing::trace!("Upgrading HTTP connection to websocket connection...");
    match ws::start(qs_ws_connection, &req, stream) {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Error upgrading connection: {}", e);
            HttpResponse::InternalServerError().body(format!("{}", e))
        }
    }
}

/// This is a wrapper for dispatch actor that can be used to send out a
/// notification over the dispatch.
#[derive(Debug)]
pub struct DispatchWebsocketNotifier {
    pub dispatch_addr: Addr<Dispatch>,
}

impl DispatchWebsocketNotifier {
    /// Create a new instance
    pub fn new(dispatch_addr: Addr<Dispatch>) -> Self {
        DispatchWebsocketNotifier { dispatch_addr }
    }

    /// Create a new instance
    pub fn default_addr() -> Self {
        let dispatch: Addr<Dispatch> = Dispatch::default().start();
        DispatchWebsocketNotifier {
            dispatch_addr: dispatch,
        }
    }
}

#[async_trait]
impl WebsocketNotifier for DispatchWebsocketNotifier {
    /// Notify a client that opened a websocket connection to the QS.
    ///
    /// # Arguments
    /// queue_id - The queue ID of the client
    ///
    /// # Returns
    ///
    /// Returns `()` of the operation was successful and
    /// `WebsocketNotifierError::ClientNotFound` if the client was not found.
    async fn notify(&self, queue_id: &QsClientId) -> Result<(), WebsocketNotifierError> {
        // Send the notification message to the dispatch actor
        self.dispatch_addr
            .send(NotifyMessage {
                queue_id: queue_id.clone(),
            })
            .await
            // If the actor doesn't reply, we get a MailboxError
            .map_err(|_| WebsocketNotifierError::WebsocketNotFound)
            // Return value of the actor
            .and_then(|res| res.map_err(|_| WebsocketNotifierError::WebsocketNotFound))
    }
}
