// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use actix::{
    ResponseFuture,
    prelude::{Actor, Context, Handler, Recipient},
};
use phnxprotos::{
    convert::RefInto,
    queue_service::v1::{ListenResponse, QueueEvent, QueueUpdate, listen_response},
};
use phnxtypes::{
    identifiers::QsClientId,
    messages::client_ds::{DsEventMessage, QsWsMessage},
};
use tokio::sync::mpsc;
use tracing::info;

use super::{
    InternalQsWsMessage,
    messages::{Connect, Disconnect, GrpcConnect, NotifyMessage, NotifyMessageError},
};

enum NotifyClientError {
    ClientNotFound,
}

enum DispatchDestination {
    Actor(Recipient<InternalQsWsMessage>),
    Channel(mpsc::UnboundedSender<ListenResponse>),
}

/// Dispatch for all websocket connections. It keeps a list of all connected
/// clients and can send messages to them.
#[derive(Default)]
pub struct Dispatch {
    sessions: HashMap<QsClientId, DispatchDestination>,
}

impl Dispatch {
    /// Notifies a connected client by sending a [`QsWsMessage::NewMessage`] to it.
    fn notify_client(
        &mut self,
        queue_id: &QsClientId,
        message: InternalQsWsMessage,
    ) -> Result<(), NotifyClientError> {
        match self.sessions.get(queue_id) {
            Some(DispatchDestination::Actor(recipient)) => {
                recipient.do_send(message);
                Ok(())
            }
            Some(DispatchDestination::Channel(tx)) => {
                if tx.send(message.into()).is_ok() {
                    Ok(())
                } else {
                    self.sessions.remove(queue_id);
                    info!("failed to notify client via websocket: channel closed");
                    Err(NotifyClientError::ClientNotFound)
                }
            }
            None => {
                info!("failed to notify client via websocket: no session");
                Err(NotifyClientError::ClientNotFound)
            }
        }
    }
}

impl From<InternalQsWsMessage> for ListenResponse {
    fn from(message: InternalQsWsMessage) -> Self {
        let response = match message.inner {
            QsWsMessage::QueueUpdate => listen_response::Response::Update(QueueUpdate {}),
            QsWsMessage::Event(DsEventMessage {
                group_id,
                sender_index,
                epoch,
                timestamp,
                payload,
            }) => listen_response::Response::Event(QueueEvent {
                group_id: Some(group_id.ref_into()),
                sender: Some(sender_index.into()),
                epoch: Some(epoch.into()),
                timestamp: Some(timestamp.into()),
                payload,
            }),
        };
        ListenResponse {
            response: Some(response),
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
        self.sessions
            .insert(msg.own_queue_id, DispatchDestination::Actor(msg.addr));
    }
}

impl Handler<GrpcConnect> for Dispatch {
    type Result = ();

    fn handle(&mut self, msg: GrpcConnect, _: &mut Self::Context) -> Self::Result {
        self.sessions
            .insert(msg.own_queue_id, DispatchDestination::Channel(msg.tx));
    }
}

// Handle Disconnect messages
impl Handler<Disconnect> for Dispatch {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.queue_id);
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
