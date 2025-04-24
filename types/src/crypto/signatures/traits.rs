// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{
    openmls::prelude::{OpenMlsCrypto, OpenMlsProvider},
    openmls_rust_crypto::OpenMlsRustCrypto,
};
use thiserror::Error;

use super::{DEFAULT_SIGNATURE_SCHEME, private_keys::VerifyingKey};
use crate::LibraryError;

use super::signable::Signature;

pub trait SigningKeyBehaviour: AsRef<super::private_keys::SigningKey> {
    /// Sign the given payload with this signing key.
    fn sign(&self, payload: &[u8]) -> Result<Signature, LibraryError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .sign(
                DEFAULT_SIGNATURE_SCHEME,
                payload,
                self.as_ref().expose_secret(),
            )
            .map_err(|_| LibraryError)
            .map(Signature::from_bytes)
    }
}

/// Error verifying signature.
#[derive(Error, Debug)]
pub enum SignatureVerificationError {
    /// Could not verify this signature with the given payload.
    #[error("Could not verify this mac with the given payload.")]
    VerificationFailure,
    /// Unrecoverable implementation error
    #[error(transparent)]
    LibraryError(#[from] LibraryError),
}

pub const SIGNATURE_PUBLIC_KEY_SIZE: usize = 32;

pub trait VerifyingKeyBehaviour: AsRef<VerifyingKey> + std::fmt::Debug {
    /// Verify the given signature with the given payload. Returns an error if the
    /// verification fails or if the signature does not have the right length.
    fn verify(&self, payload: &[u8], signature: &[u8]) -> Result<(), SignatureVerificationError> {
        let rust_crypto = OpenMlsRustCrypto::default();
        rust_crypto
            .crypto()
            .verify_signature(
                DEFAULT_SIGNATURE_SCHEME,
                payload,
                self.as_ref().as_slice(),
                signature,
            )
            .map_err(|_| SignatureVerificationError::VerificationFailure)
    }
}
