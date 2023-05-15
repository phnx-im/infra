use actix::prelude::{Message, Recipient};
use phnxbackend::qs::QsClientId;

use super::QsWsMessage;

/// Connect message for the [`Dispatch`] actor.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<QsWsMessage>,
    pub own_queue_id: QsClientId,
}

/// Disconnect message for the [`Dispatch`] actor.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub queue_id: QsClientId,
}

pub enum NotifyMessageError {
    ClientNotFound,
}

/// Notify message for the [`Dispatch`] actor. This message has a custom return
/// value because it needs to return a `Result`.
#[derive(Message)]
#[rtype(result = "Result<(), NotifyMessageError>")]
pub struct NotifyMessage {
    pub queue_id: QsClientId,
}
