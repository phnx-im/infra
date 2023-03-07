//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use mls_assist::messages::SerializedAssistedMessage;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::qs::QsClientReference;

#[derive(TlsSerialize, TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: SerializedAssistedMessage,
    pub client_reference: QsClientReference,
}
