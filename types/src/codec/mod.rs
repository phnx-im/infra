// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use cbor::Cbor;
use error::CodecError;
use mls_assist::memory_provider::Codec;
use serde::{Serialize, de::DeserializeOwned};

mod cbor;
mod error;
#[cfg(test)]
mod tests;

pub use error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
#[non_exhaustive]
pub enum PhnxCodec {
    #[cfg(test)]
    OlderTestVersion = 0,
    #[default]
    V1 = 1,
}

impl TryFrom<u8> for PhnxCodec {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            #[cfg(test)]
            0 => Ok(PhnxCodec::OlderTestVersion),
            1 => Ok(PhnxCodec::V1),
            _ => Err(Error::UnknownCodecVersion),
        }
    }
}

impl PhnxCodec {
    fn serialize<T: Sized + Serialize>(
        &self,
        value: &T,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let res = match self {
            #[cfg(test)]
            PhnxCodec::OlderTestVersion => tests::Json::to_vec(value)?,
            PhnxCodec::V1 => Cbor::to_vec(value)?,
        };

        // The first byte is always the codec version
        let mut buf = vec![*self as u8];
        buf.extend(res);
        Ok(buf)
    }

    fn deserialize<T: DeserializeOwned>(
        &self,
        bytes: &[u8],
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let res = match self {
            #[cfg(test)]
            PhnxCodec::OlderTestVersion => tests::Json::from_slice(bytes)?,
            PhnxCodec::V1 => Cbor::from_slice(bytes)?,
        };
        Ok(res)
    }

    pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
    where
        T: Sized + Serialize,
    {
        let codec_version = PhnxCodec::default();
        let res = codec_version.serialize(value).map_err(|error| CodecError {
            codec_version,
            error,
        })?;
        Ok(res)
    }

    pub fn from_slice<T>(bytes: &[u8]) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let codec_version_byte = bytes.first().ok_or(Error::EmptyyInputSlice)?;
        let codec_version = PhnxCodec::try_from(*codec_version_byte)?;
        codec_version.deserialize(&bytes[1..]).map_err(|error| {
            CodecError {
                codec_version,
                error,
            }
            .into()
        })
    }
}

impl Codec for PhnxCodec {
    type Error = Error;

    fn to_vec<T>(value: &T) -> Result<Vec<u8>, Self::Error>
    where
        T: Sized + Serialize,
    {
        Self::to_vec(value)
    }

    fn from_slice<T>(bytes: &[u8]) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        Self::from_slice(bytes)
    }
}
