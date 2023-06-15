// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserializeBytes, TlsSize};

use crate::qs::QsClientReference;

use super::client_ds::{AssistedMessagePlus, EventMessage, QsQueueMessagePayload};

// === DS to QS ===

#[derive(TlsDeserializeBytes, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: DsFanOutPayload,
    pub client_reference: QsClientReference,
}

#[derive(Clone, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum DsFanOutPayload {
    QueueMessage(QsQueueMessagePayload),
    EventMessage(EventMessage),
}

impl From<AssistedMessagePlus> for DsFanOutPayload {
    fn from(value: AssistedMessagePlus) -> Self {
        Self::QueueMessage(value.into())
    }
}
