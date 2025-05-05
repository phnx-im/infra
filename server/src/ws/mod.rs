// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod dispatch;
pub(crate) mod messages;

use actix::{Actor, Addr, Message};
use dispatch::*;
use messages::*;
use phnxbackend::qs::{
    WebsocketNotifier, WebsocketNotifierError, WsNotification, grpc::GrpcListen,
};
use phnxprotos::queue_service::v1::QueueEvent;
use phnxtypes::{identifiers::QsClientId, messages::client_ds::QsWsMessage};
use tokio::{self, sync::mpsc};
use tracing::error;

// Type for internal use so we can derive `Message` and use the rtype attribute.
#[derive(PartialEq, Eq, Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct InternalQsWsMessage {
    inner: QsWsMessage,
}

impl From<QsWsMessage> for InternalQsWsMessage {
    fn from(message: QsWsMessage) -> Self {
        InternalQsWsMessage { inner: message }
    }
}

impl From<WsNotification> for InternalQsWsMessage {
    fn from(notification: WsNotification) -> Self {
        match notification {
            WsNotification::QueueUpdate => QsWsMessage::QueueUpdate,
            WsNotification::Event(event) => QsWsMessage::Event(event),
        }
        .into()
    }
}

pub struct Client {
    pub queue_id: QsClientId,
}

/// This is a wrapper for dispatch actor that can be used to send out a
/// notification over the dispatch.
#[derive(Clone, Debug)]
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

impl WebsocketNotifier for DispatchWebsocketNotifier {
    /// Notify a client that opened a websocket connection to the QS.
    ///
    /// # Arguments
    /// queue_id - The queue ID of the client
    /// ws_notification - The notification to send
    ///
    /// # Returns
    ///
    /// Returns `()` of the operation was successful and
    /// `WebsocketNotifierError::ClientNotFound` if the client was not found.
    async fn notify(
        &self,
        queue_id: &QsClientId,
        ws_notification: WsNotification,
    ) -> Result<(), WebsocketNotifierError> {
        // Send the notification message to the dispatch actor
        self.dispatch_addr
            .send(NotifyMessage {
                queue_id: *queue_id,
                payload: ws_notification.into(),
            })
            .await
            // If the actor doesn't reply, we get a MailboxError
            .map_err(|e| {
                tracing::warn!(
                    "Got a MailboxError while trying to send a message to the WS actor: {}",
                    e
                );
                WebsocketNotifierError::WebsocketNotFound
            })
            // Return value of the actor
            .and_then(|res| res.map_err(|e| {
                tracing::warn!("The WS actor returned the following error while trying to send a message via WS: {:?}", e);
                WebsocketNotifierError::WebsocketNotFound}))
    }
}

impl GrpcListen for DispatchWebsocketNotifier {
    async fn register_connection(
        &self,
        queue_id: QsClientId,
        tx: mpsc::UnboundedSender<QueueEvent>,
    ) {
        if let Err(error) = self
            .dispatch_addr
            .send(GrpcConnect {
                own_queue_id: queue_id,
                tx,
            })
            .await
        {
            error!(%error, "failed to register grpc connection");
        }
    }
}
