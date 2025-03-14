// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::crypto::ear::{EarDecryptable, EarEncryptable, keys::PushTokenEarKey};

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
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Clone,
    Debug,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct EncryptedPushToken(Ciphertext);

impl AsRef<Ciphertext> for EncryptedPushToken {
    fn as_ref(&self) -> &Ciphertext {
        &self.0
    }
}

impl From<Ciphertext> for EncryptedPushToken {
    fn from(ctxt: Ciphertext) -> Self {
        Self(ctxt)
    }
}

impl EarEncryptable<PushTokenEarKey, EncryptedPushToken> for PushToken {}
impl EarDecryptable<PushTokenEarKey, EncryptedPushToken> for PushToken {}
