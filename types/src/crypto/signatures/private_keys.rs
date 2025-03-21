// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::SignaturePublicKey,
    openmls_rust_crypto::OpenMlsRustCrypto,
    openmls_traits::{OpenMlsProvider, crypto::OpenMlsCrypto},
};
use serde::{Deserialize, Serialize};
use tls_codec::{TlsDeserializeBytes, TlsSerialize, TlsSize};

use crate::crypto::{errors::KeyGenerationError, secrets::SecretBytes};

use super::DEFAULT_SIGNATURE_SCHEME;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    TlsSerialize,
    TlsDeserializeBytes,
    TlsSize,
    PartialEq,
    Eq,
    sqlx::Type,
)]
#[sqlx(transparent)]
pub struct VerifyingKey(Vec<u8>);

// We need these traits to interop the MLS leaf keys.
impl From<SignaturePublicKey> for VerifyingKey {
    fn from(pk: SignaturePublicKey) -> Self {
        Self(pk.as_slice().to_vec())
    }
}

impl From<VerifyingKey> for SignaturePublicKey {
    fn from(pk: VerifyingKey) -> Self {
        SignaturePublicKey::from(pk.0)
    }
}

impl VerifyingKey {
    #[cfg(any(test, feature = "test_utils"))]
    pub fn new_for_test(value: Vec<u8>) -> Self {
        Self(value)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "signing_key_data")]
pub struct SigningKey {
    signing_key: SecretBytes,
    verifying_key: VerifyingKey,
}

impl SigningKey {
    pub fn generate() -> Result<SigningKey, KeyGenerationError> {
        let (private_key, public_key) = OpenMlsRustCrypto::default()
            .crypto()
            .signature_key_gen(DEFAULT_SIGNATURE_SCHEME)
            .map_err(|_| KeyGenerationError::KeypairGeneration)?;
        Ok(Self {
            signing_key: SecretBytes::from(private_key),
            verifying_key: VerifyingKey(public_key),
        })
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    pub(super) fn expose_secret(&self) -> &SecretBytes {
        &self.signing_key
    }
}
