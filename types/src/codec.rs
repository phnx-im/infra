// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Serialization(#[from] ciborium::ser::Error<std::io::Error>),
    #[error(transparent)]
    Deserialization(#[from] ciborium::de::Error<std::io::Error>),
}

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: Sized + Serialize,
{
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)?;
    Ok(buf)
}

pub fn from_slice<T>(bytes: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    Ok(ciborium::de::from_reader(bytes)?)
}
