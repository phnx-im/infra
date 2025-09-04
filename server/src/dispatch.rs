// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Arc};

use airbackend::qs::{Notification, Notifier, NotifierError, grpc::GrpcListen};
use aircommon::{identifiers::QsClientId, messages::client_ds::DsEventMessage};
use airprotos::{
    convert::RefInto,
    queue_service::v1::{QueueEvent, QueueEventPayload, QueueEventUpdate, queue_event},
};
use tokio::{
    self,
    sync::{Mutex, mpsc},
};
use tracing::info;

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
        notification: Notification,
    ) -> Result<(), NotifierError> {
        let mut dispatch = self.dispatch.lock().await;
        dispatch.notify_client(queue_id, notification)
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

/// Dispatch for all connections.
///
/// It keeps a list of all connected clients and can send messages to them.
#[derive(Default, Debug)]
pub struct Dispatch {
    sessions: HashMap<QsClientId, mpsc::UnboundedSender<QueueEvent>>,
}

impl Dispatch {
    /// Notifies a connected client by sending a [`QsWsMessage::NewMessage`] to it.
    pub(crate) fn notify_client(
        &mut self,
        queue_id: &QsClientId,
        message: Notification,
    ) -> Result<(), NotifierError> {
        match self.sessions.get(queue_id) {
            Some(tx) => {
                if tx.send(convert_qs_message(message)).is_ok() {
                    Ok(())
                } else {
                    self.sessions.remove(queue_id);
                    info!("failed to notify client via websocket: channel closed");
                    Err(NotifierError::ClientNotFound)
                }
            }
            None => {
                info!("failed to notify client via websocket: no session");
                Err(NotifierError::ClientNotFound)
            }
        }
    }

    pub(crate) fn connect(&mut self, queue_id: QsClientId, tx: mpsc::UnboundedSender<QueueEvent>) {
        self.sessions.insert(queue_id, tx);
    }
}

fn convert_qs_message(message: Notification) -> QueueEvent {
    let event = match message {
        Notification::QueueUpdate => queue_event::Event::Update(QueueEventUpdate {}),
        Notification::Event(DsEventMessage {
            group_id,
            sender_index,
            epoch,
            timestamp,
            payload,
        }) => queue_event::Event::Payload(QueueEventPayload {
            group_id: Some(group_id.ref_into()),
            sender: Some(sender_index.into()),
            epoch: Some(epoch.into()),
            timestamp: Some(timestamp.into()),
            payload,
        }),
    };
    QueueEvent { event: Some(event) }
}
