// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::crypto::ear::{keys::PushTokenEarKey, EarDecryptable, EarEncryptable};

use super::*;

#[derive(Serialize, Deserialize)]
pub enum PushTokenOperator {
    Apple,
    Google,
}

#[derive(Serialize, Deserialize)]
pub struct PushToken {
    operator: PushTokenOperator,
    token: String,
}

impl PushToken {
    /// Create a new push token.
    pub fn new(operator: PushTokenOperator, token: String) -> Self {
        Self { operator, token }
    }

    pub fn operator(&self) -> &PushTokenOperator {
        &self.operator
    }

    pub fn token(&self) -> &str {
        &self.token
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
