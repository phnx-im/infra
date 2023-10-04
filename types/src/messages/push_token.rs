// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::crypto::ear::{keys::PushTokenEarKey, EarDecryptable, EarEncryptable};

use super::*;

#[derive(Serialize, Deserialize)]
pub struct PushToken {
    token: Vec<u8>,
}

impl PushToken {
    // TODO: This is a dummy implementation for now
    pub fn dummy() -> Self {
        Self { token: vec![0; 32] }
    }

    /// If the alert level is high enough, send a notification to the client.
    pub fn send_notification(&self, _alert_level: u8) {
        todo!()
    }
}
#[derive(
    Serialize, Deserialize, PartialEq, Clone, Debug, TlsSerialize, TlsDeserializeBytes, TlsSize,
)]
pub struct EncryptedPushToken {
    ctxt: Ciphertext,
}

impl AsRef<Ciphertext> for EncryptedPushToken {
    fn as_ref(&self) -> &Ciphertext {
        &self.ctxt
    }
}

impl From<Ciphertext> for EncryptedPushToken {
    fn from(ctxt: Ciphertext) -> Self {
        Self { ctxt }
    }
}

impl EarEncryptable<PushTokenEarKey, EncryptedPushToken> for PushToken {}
impl EarDecryptable<PushTokenEarKey, EncryptedPushToken> for PushToken {}
