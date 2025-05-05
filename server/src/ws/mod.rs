// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod dispatch;
pub(crate) mod messages;

use std::sync::Arc;

use actix::Message;
use dispatch::*;
use phnxbackend::qs::{Notifier, WebsocketNotifierError, WsNotification, grpc::GrpcListen};
use phnxprotos::queue_service::v1::QueueEvent;
use phnxtypes::{identifiers::QsClientId, messages::client_ds::QsWsMessage};
use tokio::{
    self,
    sync::{Mutex, mpsc},
};

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

/// This is a wrapper for dispatch actor that can be used to send out a
/// notification over the dispatch.
#[derive(Clone, Debug, Default)]
pub struct DispatchNotifier {
    pub dispatch: Arc<Mutex<Dispatch>>,
}

impl DispatchNotifier {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Notifier for DispatchNotifier {
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
        let mut dispatch = self.dispatch.lock().await;
        match dispatch.notify_client(queue_id, ws_notification.into()) {
            Ok(_) => Ok(()),
            Err(NotifyClientError::ClientNotFound) => {
                Err(WebsocketNotifierError::WebsocketNotFound)
            }
        }
    }
}

impl GrpcListen for DispatchNotifier {
    async fn register_connection(
        &self,
        queue_id: QsClientId,
        tx: mpsc::UnboundedSender<QueueEvent>,
    ) {
        let mut dispatch = self.dispatch.lock().await;
        dispatch.connect(queue_id, tx);
    }
}
