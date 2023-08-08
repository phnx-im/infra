// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::qs::Fqdn;

use super::{intra_backend::DsFanOutMessage, MlsInfraVersion};

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
#[repr(u8)]
pub enum QsToQsPayload {
    FanOutMessageRequest(DsFanOutMessage),
    VerificationKeyRequest,
}

#[derive(TlsSerialize, TlsDeserializeBytes, TlsSize)]
pub struct QsToQsMessage {
    pub protocol_version: MlsInfraVersion,
    pub sender: Fqdn,
    pub recipient: Fqdn,
    pub payload: QsToQsPayload,
    // TODO: Signature
}
