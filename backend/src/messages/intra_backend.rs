// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserialize, TlsSize};

use crate::qs::QsClientReference;

use super::client_ds::QueueMessagePayload;

// === DS to QS ===

#[derive(TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: QueueMessagePayload,
    pub client_reference: QsClientReference,
}
