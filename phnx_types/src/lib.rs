// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tls_codec::{
    DeserializeBytes as TlsDeserializeBytesTrait, Serialize as TlsSerializeTrait, Size,
    TlsDeserializeBytes, TlsSerialize, TlsSize,
};

pub mod credentials;
pub mod crypto;
pub mod identifiers;
pub mod keypackage_batch;
pub mod messages;
pub mod time;

/// Unrecoverable error in this implementation.
#[derive(Debug, Error, Serialize, Deserialize)]
pub struct LibraryError;

impl LibraryError {
    pub fn missing_bound_check(_error: tls_codec::Error) -> Self {
        LibraryError {}
    }

    pub fn unexpected_crypto_error(_error: &str) -> Self {
        LibraryError {}
    }
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
