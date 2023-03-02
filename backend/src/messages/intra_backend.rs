//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::qs::ClientQueueConfig;

use super::client_backend::ClientToClientMsg;

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: ClientToClientMsg,
    pub queue_config: ClientQueueConfig,
}
