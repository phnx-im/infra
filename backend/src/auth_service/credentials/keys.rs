// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::openmls::prelude::{
    OpenMlsCrypto, OpenMlsCryptoProvider, SignaturePublicKey, SignatureScheme,
};
use mls_assist::openmls_rust_crypto::OpenMlsRustCrypto;
use tls_codec::{TlsDeserialize, TlsSerialize, TlsSize};

use crate::auth_service::credentials::{
    AsCredential, AsIntermediateCredential, PreliminaryAsSigningKey,
};

use crate::crypto::signatures::traits::{SigningKey, VerifyingKey};

use thiserror::Error;

use super::{ClientCredential, PreliminaryClientSigningKey};

#[derive(Debug)]
pub struct AsIntermediateSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: AsIntermediateCredential,
}

impl AsRef<[u8]> for AsIntermediateSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl SigningKey for AsIntermediateSigningKey {}

impl AsIntermediateSigningKey {
    pub(super) fn from_prelim_key(
        prelim_key: PreliminaryAsSigningKey,
        credential: AsIntermediateCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key_bytes: prelim_key.into_signing_key_bytes(),
            credential,
        })
    }

    pub(crate) fn credential(&self) -> &AsIntermediateCredential {
        &self.credential
    }
}

pub(super) enum SigningKeyCreationError {
    PublicKeyMismatch,
}

#[derive(Debug)]
pub(crate) struct AsSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: AsCredential,
}

impl AsRef<[u8]> for AsSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl AsSigningKey {
    pub(super) fn from_bytes_and_credential(
        signing_key_bytes: Vec<u8>,
        credential: AsCredential,
    ) -> Self {
        Self {
            signing_key_bytes,
            credential,
        }
    }

    pub(crate) fn credential(&self) -> &AsCredential {
        &self.credential
    }
}

impl SigningKey for AsSigningKey {}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize)]
pub(super) struct AsVerifyingKey {
    verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for AsVerifyingKey {}

impl AsRef<[u8]> for AsVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

impl From<Vec<u8>> for AsVerifyingKey {
    fn from(value: Vec<u8>) -> Self {
        Self {
            verifying_key_bytes: value.into(),
        }
    }
}

/// Generates a tuple consisting of private and public key.
pub(super) fn generate_signature_keypair() -> Result<(Vec<u8>, Vec<u8>), KeyGenerationError> {
    OpenMlsRustCrypto::default()
        .crypto()
        .signature_key_gen(SignatureScheme::ED25519)
        .map_err(|_| KeyGenerationError::KeypairGeneration)
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, Eq, PartialEq)]
pub(super) struct AsIntermediateVerifyingKey {
    pub(super) verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for AsIntermediateVerifyingKey {}

impl AsRef<[u8]> for AsIntermediateVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub(crate) enum KeyGenerationError {
    /// Error generating signature keypair
    #[error("Error generating signature keypair")]
    KeypairGeneration,
}

#[derive(Debug)]
pub struct ClientSigningKey {
    signing_key_bytes: Vec<u8>,
    credential: ClientCredential,
}

impl AsRef<[u8]> for ClientSigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.signing_key_bytes
    }
}

impl SigningKey for ClientSigningKey {}

impl ClientSigningKey {
    pub(super) fn from_prelim_key(
        prelim_key: PreliminaryClientSigningKey,
        credential: ClientCredential,
    ) -> Result<Self, SigningKeyCreationError> {
        if &prelim_key.verifying_key != credential.verifying_key() {
            return Err(SigningKeyCreationError::PublicKeyMismatch);
        }
        Ok(Self {
            signing_key_bytes: prelim_key.into_signing_key_bytes(),
            credential,
        })
    }

    pub fn credential(&self) -> &ClientCredential {
        &self.credential
    }
}

#[derive(Clone, Debug, TlsSerialize, TlsDeserialize, TlsSize, Eq, PartialEq)]
pub struct ClientVerifyingKey {
    pub(super) verifying_key_bytes: SignaturePublicKey,
}

impl VerifyingKey for ClientVerifyingKey {}

impl AsRef<[u8]> for ClientVerifyingKey {
    fn as_ref(&self) -> &[u8] {
        self.verifying_key_bytes.as_slice()
    }
}
