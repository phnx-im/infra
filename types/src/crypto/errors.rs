// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum KeyGenerationError {
    /// Error generating signature keypair
    #[error("Error generating signature keypair")]
    KeypairGeneration,
}

#[derive(Debug, Error)]
pub enum RandomnessError {
    #[error("Insufficient randomness")]
    InsufficientRandomness,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum UnsealError {
    /// Decryption error
    #[error("Decryption error")]
    DecryptionError,
    /// Codec error
    #[error("Codec error")]
    CodecError,
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum SealError {
    /// Encryption error
    #[error("Encryption error")]
    EncryptionError,
    /// Codec error
    #[error("Codec error")]
    CodecError,
}
