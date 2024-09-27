// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Deref;

use tls_codec::{DeserializeBytes, Error, Serialize, Size};
use url::Host;
use uuid::Uuid;

use super::{Fqdn, UserName};

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[serde(transparent)]
pub(super) struct TlsUuid(pub Uuid);

impl Deref for TlsUuid {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Size for TlsUuid {
    fn tls_serialized_len(&self) -> usize {
        self.0.as_bytes().len()
    }
}

impl Serialize for TlsUuid {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, Error> {
        self.0.as_bytes().tls_serialize(writer)
    }
}

impl DeserializeBytes for TlsUuid {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error>
    where
        Self: Sized,
    {
        let (uuid_bytes, rest) = <[u8; 16]>::tls_deserialize_bytes(bytes)?;
        let uuid = Uuid::from_bytes(uuid_bytes);
        Ok((Self(uuid), rest))
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(transparent)]
pub(super) struct TlsStr<'a>(pub &'a str);

impl Size for TlsStr<'_> {
    fn tls_serialized_len(&self) -> usize {
        self.0.as_bytes().tls_serialized_len()
    }
}

impl Serialize for TlsStr<'_> {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, Error> {
        self.0.as_bytes().tls_serialize(writer)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[serde(transparent)]
pub(super) struct TlsString(pub String);

impl std::fmt::Display for TlsString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Size for TlsString {
    fn tls_serialized_len(&self) -> usize {
        TlsStr(&self.0).tls_serialized_len()
    }
}

impl DeserializeBytes for TlsString {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error>
    where
        Self: Sized,
    {
        let (string, rest) = <Vec<u8>>::tls_deserialize_bytes(bytes)?;
        let string = String::from_utf8(string)
            .map_err(|_| Error::DecodingError("Couldn't decode string.".to_owned()))?;
        Ok((Self(string), rest))
    }
}

impl Serialize for TlsString {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, Error> {
        TlsStr(&self.0).tls_serialize(writer)
    }
}

impl Size for Fqdn {
    fn tls_serialized_len(&self) -> usize {
        if let Host::Domain(domain) = &self.domain {
            TlsStr(domain.as_str()).tls_serialized_len()
        } else {
            0
        }
    }
}

impl DeserializeBytes for Fqdn {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error>
    where
        Self: Sized,
    {
        let (TlsString(domain_string), rest) = TlsString::tls_deserialize_bytes(bytes)?;
        let domain = Fqdn::try_from(domain_string).map_err(|e| {
            let e = format!("Couldn't decode domain string: {}.", e);
            Error::DecodingError(e)
        })?;
        Ok((domain, rest))
    }
}

impl Serialize for Fqdn {
    fn tls_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, Error> {
        if let Host::Domain(domain) = &self.domain {
            TlsStr(domain.as_str()).tls_serialize(writer)
        } else {
            Ok(0)
        }
    }
}

impl DeserializeBytes for UserName {
    fn tls_deserialize_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error>
    where
        Self: Sized,
    {
        let (TlsString(user_name_string), rest) = TlsString::tls_deserialize_bytes(bytes)?;
        let user_name = UserName::try_from(user_name_string).map_err(|e| {
            let e = format!("Couldn't decode user name string: {}.", e);
            Error::DecodingError(e)
        })?;
        Ok((user_name, rest))
    }
}
