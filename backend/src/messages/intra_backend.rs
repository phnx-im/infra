//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserialize, TlsSize};

use crate::qs::ClientQueueConfig;

use super::client_ds::ClientToClientMsg;

#[derive(TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: ClientToClientMsg,
    pub queue_config: ClientQueueConfig,
}
