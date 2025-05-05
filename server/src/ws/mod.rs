// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod dispatch;

use std::sync::Arc;

use dispatch::*;
use phnxbackend::qs::{Notification, Notifier, NotifierError, grpc::GrpcListen};
use phnxprotos::queue_service::v1::QueueEvent;
use phnxtypes::identifiers::QsClientId;
use tokio::{
    self,
    sync::{Mutex, mpsc},
};

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
