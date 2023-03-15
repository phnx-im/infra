// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! This module contains structs and enums that represent messages that are
//! passed internally within the backend.

use tls_codec::{TlsDeserialize, TlsSize};

use crate::{auth_service::AsClientId, qs::QsClientReference};

use super::client_ds::DsFanoutPayload;

// === DS to QS ===

#[derive(TlsDeserialize, TlsSize)]
pub struct DsFanOutMessage {
    pub payload: DsFanoutPayload,
    pub client_reference: QsClientReference,
}

// === DS to AS ===

pub(crate) struct AsEnqueueMessageParams {
    pub(crate) client_id: AsClientId,
    pub(crate) connection_establishment_ctxt: Vec<u8>,
}
