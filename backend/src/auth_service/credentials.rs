// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use super::AsClientId;

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsCredentials {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct AsIntermediateCredential {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub(crate) struct Fingerprint {}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientCsr {}

impl ClientCsr {
    pub fn identity(&self) -> AsClientId {
        todo!()
    }

    pub fn validate(&self) -> bool {
        todo!()
    }
}

#[derive(Debug, TlsDeserialize, TlsSerialize, TlsSize)]
pub struct ClientCredential {}

impl ClientCredential {
    pub fn new_from_csr(csr: ClientCsr) -> Self {
        // TODO: this needs to be signed by the AS
        todo!()
    }

    pub fn identity(&self) -> AsClientId {
        todo!()
    }
}
