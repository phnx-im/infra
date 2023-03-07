//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserialize, TlsSize};

use crate::qs::QsClientReference;

use super::client_ds::ClientToClientMsg;

#[derive(TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: ClientToClientMsg,
    pub client_reference: QsClientReference,
}
