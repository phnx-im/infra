// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

use super::Codec;

#[derive(Debug)]
pub(super) struct Cbor;

impl Cbor {
    pub(crate) fn to_writer(
        value: &impl Serialize,
        writer: &mut impl std::io::Write,
    ) -> Result<(), ciborium::ser::Error<std::io::Error>> {
        ciborium::into_writer(value, writer)
    }
}

#[derive(Debug, Error)]
pub enum CborError {
    #[error(transparent)]
    Serialization(#[from] ciborium::ser::Error<std::io::Error>),
    #[error(transparent)]
    Deserialization(#[from] ciborium::de::Error<std::io::Error>),
}

impl Codec for Cbor {
    type Error = CborError;

    fn to_vec<T>(value: &T) -> Result<Vec<u8>, Self::Error>
    where
        T: Sized + Serialize,
    {
        let mut buf = Vec::new();
        ciborium::into_writer(value, &mut buf)?;
        Ok(buf)
    }

    fn from_slice<T>(bytes: &[u8]) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        Ok(ciborium::de::from_reader(bytes)?)
    }
}
