// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{
    messages::{Connect, Disconnect, NotifyMessage, NotifyMessageError},
    QsWsMessage,
};
use actix::{
    prelude::{Actor, Context, Handler, Recipient},
    ResponseFuture,
};
use phnxbackend::qs::QsClientId;

use std::collections::HashMap;

enum NotifyClientError {
    ClientNotFound,
}

/// Dispatch for all websocket connections. It keeps a list of all connected
/// clients and can send messages to them.
#[derive(Default)]
pub struct Dispatch {
    sessions: HashMap<QsClientId, Recipient<QsWsMessage>>,
}

impl Dispatch {
    /// Notifies a connected client by sending a [`QsWsMessage::NewMessage`] to it.
    fn notify_client(
        &self,
        queue_id: &QsClientId,
        message: QsWsMessage,
    ) -> Result<(), NotifyClientError> {
        if let Some(socket_recipient) = self.sessions.get(queue_id) {
            socket_recipient.do_send(message);
            Ok(())
        } else {
            // This is only rated "info", because not having a websocket open is
            // not irregular.
            tracing::info!("Failed to notify client via websocket.");
            Err(NotifyClientError::ClientNotFound)
        }
    }
}

// Makes Dispatch an Actor
impl Actor for Dispatch {
    type Context = Context<Self>;
}

// Handle Connect messages
impl Handler<Connect> for Dispatch {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.own_queue_id, msg.addr);
    }
}

// Handle Disconnect messages
impl Handler<Disconnect> for Dispatch {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        if self.sessions.remove(&msg.queue_id).is_some() {}
    }
}

// Handle Notify messages
impl Handler<NotifyMessage> for Dispatch {
    type Result = ResponseFuture<Result<(), NotifyMessageError>>;

    fn handle(&mut self, msg: NotifyMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match self.notify_client(&msg.queue_id, msg.payload) {
            Ok(_) => Box::pin(async { Ok(()) }),
            Err(_) => Box::pin(async { Err(NotifyMessageError::ClientNotFound) }),
        }
    }
}
