// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use mls_assist::{OpenMlsCrypto, OpenMlsCryptoProvider, OpenMlsRustCrypto, SignatureScheme};
use thiserror::Error;

use crate::LibraryError;

use super::signable::Signature;

pub trait SigningKey: AsRef<[u8]> {
    /// Sign the given payload with this signing key.
    fn sign(&self, payload: &[u8]) -> Result<Signature, LibraryError> {
        let backend = OpenMlsRustCrypto::default();
        backend
            .crypto()
            .sign(SignatureScheme::ED25519, payload, self.as_ref())
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

pub trait VerifyingKey: AsRef<[u8]> + std::fmt::Debug {
    /// Verify the given signature with the given payload. Returns an error if the
    /// verification fails or if the signature does not have the right length.
    fn verify(
        &self,
        payload: &[u8],
        signature: &Signature,
    ) -> Result<(), SignatureVerificationError> {
        let backend = OpenMlsRustCrypto::default();
        backend
            .crypto()
            .verify_signature(
                SignatureScheme::ED25519,
                payload,
                self.as_ref(),
                signature.as_slice(),
            )
            .map_err(|_| SignatureVerificationError::VerificationFailure)
    }
}
