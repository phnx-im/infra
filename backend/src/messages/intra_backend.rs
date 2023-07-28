// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use mls_assist::messages::SerializedMlsMessage;
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::qs::QsClientReference;

use super::client_ds::{EventMessage, QsQueueMessagePayload};

// === DS to QS ===

#[derive(Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: DsFanOutPayload,
    pub client_reference: QsClientReference,
}

#[derive(Clone, TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsFanOutPayload {
    QueueMessage(QsQueueMessagePayload),
    EventMessage(EventMessage),
}

impl From<SerializedMlsMessage> for DsFanOutPayload {
    fn from(value: SerializedMlsMessage) -> Self {
        Self::QueueMessage(value.into())
    }
}
