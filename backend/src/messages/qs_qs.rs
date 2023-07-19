// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::qs::Fqdn;

use super::{intra_backend::DsFanOutMessage, MlsInfraVersion};

pub struct QsToQsMessage {
    protocol_version: MlsInfraVersion,
    sender: Fqdn,
    recipient: Fqdn,
    fan_out_message: DsFanOutMessage,
    // TODO: Signature
}
