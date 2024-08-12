// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{crypto::OpenMlsCrypto, OpenMlsProvider},
};
use secrecy::{zeroize::Zeroize, CloneableSecret, DebugSecret, SerializableSecret};
use serde::{Deserialize, Serialize};

use crate::crypto::errors::KeyGenerationError;

use super::DEFAULT_SIGNATURE_SCHEME;

#[derive(Clone, Serialize, Deserialize)]
pub struct PrivateKey(Vec<u8>);

impl SerializableSecret for PrivateKey {}
impl CloneableSecret for PrivateKey {}
impl DebugSecret for PrivateKey {}

/// Generates a tuple consisting of private and public key.
pub fn generate_signature_keypair() -> Result<(PrivateKey, Vec<u8>), KeyGenerationError> {
    let (private_key, public_key) = OpenMlsRustCrypto::default()
        .crypto()
        .signature_key_gen(DEFAULT_SIGNATURE_SCHEME)
        .map_err(|_| KeyGenerationError::KeypairGeneration)?;
    Ok((PrivateKey(private_key), public_key))
}

impl PrivateKey {
    pub(super) fn expose_secret(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        PrivateKey::debug_secret(f)
    }
}

impl Zeroize for PrivateKey {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}
