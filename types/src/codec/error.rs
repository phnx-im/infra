// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

use super::PhnxCodec;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Empty input slice")]
    EmptyInputSlice,
    #[error("Invalid codec version")]
    UnknownCodecVersion,
    #[error("Codec error: {0}")]
    CodecError(#[from] CodecError),
}

#[derive(Debug, Error)]
pub struct CodecError {
    pub(super) codec_version: PhnxCodec,
    pub(super) error: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.codec_version, self.error)
    }
}

#[cfg(feature = "sqlite")]
impl From<Error> for rusqlite::types::FromSqlError {
    fn from(e: Error) -> Self {
        rusqlite::types::FromSqlError::Other(Box::new(e))
    }
}

#[cfg(feature = "sqlite")]
impl From<Error> for rusqlite::Error {
    fn from(e: Error) -> Self {
        rusqlite::Error::ToSqlConversionFailure(e.into())
    }
}
