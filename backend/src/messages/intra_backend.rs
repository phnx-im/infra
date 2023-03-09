//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserialize, TlsSize};

use crate::qs::QsClientReference;

#[derive(TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: Vec<u8>,
    pub client_reference: QsClientReference,
}
