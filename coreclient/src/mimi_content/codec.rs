// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;

use phnxtypes::identifiers::Fqdn;
use tls_codec::{DeserializeBytes, Serialize, Size};
use url::Url;
use uuid::Uuid;

use super::{ContentType, ExternalPartUrl, MessageId, SinglePart, TlsStr, TlsStrOwned};

impl Size for TlsStr<'_> {
    fn tls_serialized_len(&self) -> usize {
        self.value.as_bytes().tls_serialized_len()
    }
}

impl Serialize for TlsStr<'_> {
    fn tls_serialize<W: std::io::prelude::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, tls_codec::Error> {
        self.value.as_bytes().tls_serialize(writer)
    }
}

impl Size for TlsStrOwned {
    fn tls_serialized_len(&self) -> usize {
        TlsStr::from(self.value.as_str()).tls_serialized_len()
    }
}

impl Serialize for TlsStrOwned {
    fn tls_serialize<W: std::io::prelude::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, tls_codec::Error> {
        TlsStr::from(self.value.as_str()).tls_serialize(writer)
    }
}

impl DeserializeBytes for TlsStrOwned {
    fn tls_deserialize_bytes(buffer: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (value_bytes, buffer) = Vec::<u8>::tls_deserialize_bytes(buffer)?;
        let value = String::from_utf8(value_bytes)
            .map_err(|e| tls_codec::Error::DecodingError(format!("Invalid UTF-8: {}", e)))?;
        Ok((Self { value }, buffer))
    }
}

impl Size for MessageId {
    fn tls_serialized_len(&self) -> usize {
        // Uuid is 16 bytes
        16 + self.domain.tls_serialized_len()
    }
}

impl Serialize for MessageId {
    fn tls_serialize<W: std::io::prelude::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, tls_codec::Error> {
        let mut written = writer.write(self.id.as_bytes())?;
        written += self.domain.tls_serialize(writer)?;
        Ok(written)
    }
}

impl DeserializeBytes for MessageId {
    fn tls_deserialize_bytes(buffer: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (id_bytes, buffer) = <[u8; 16]>::tls_deserialize_bytes(buffer)?;
        let id = Uuid::from_bytes(id_bytes);
        let (domain, buffer) = Fqdn::tls_deserialize_bytes(buffer)?;
        Ok((Self { id, domain }, buffer))
    }
}

impl Size for ExternalPartUrl {
    fn tls_serialized_len(&self) -> usize {
        TlsStr::from(self.url.as_str()).tls_serialized_len()
    }
}

impl Serialize for ExternalPartUrl {
    fn tls_serialize<W: std::io::prelude::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, tls_codec::Error> {
        TlsStr::from(self.url.as_str()).tls_serialize(writer)
    }
}

impl DeserializeBytes for ExternalPartUrl {
    fn tls_deserialize_bytes(buffer: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (url, buffer) = TlsStrOwned::tls_deserialize_bytes(buffer)?;
        Ok((
            Self {
                url: Url::from_str(&url.value)
                    .map_err(|e| tls_codec::Error::DecodingError(format!("Invalid URL: {}", e)))?,
            },
            buffer,
        ))
    }
}

impl Size for ContentType {
    fn tls_serialized_len(&self) -> usize {
        match self {
            ContentType::TextMarkdown => TlsStr::from("text/markdown").tls_serialized_len(),
        }
    }
}

impl Serialize for ContentType {
    fn tls_serialize<W: std::io::prelude::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, tls_codec::Error> {
        match self {
            ContentType::TextMarkdown => TlsStr::from("text/markdown").tls_serialize(writer),
        }
    }
}

impl DeserializeBytes for ContentType {
    fn tls_deserialize_bytes(buffer: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (value, buffer) = TlsStrOwned::tls_deserialize_bytes(buffer)?;
        match value.value.as_str() {
            "text/markdown" => Ok((ContentType::TextMarkdown, buffer)),
            _ => Err(tls_codec::Error::DecodingError(format!(
                "Unknown content type: {}",
                value.value
            ))),
        }
    }
}

impl Size for SinglePart {
    fn tls_serialized_len(&self) -> usize {
        match self {
            SinglePart::TextMarkdown(content) => {
                ContentType::TextMarkdown.tls_serialized_len()
                    + content.as_bytes().tls_serialized_len()
            }
        }
    }
}

impl Serialize for SinglePart {
    fn tls_serialize<W: std::io::prelude::Write>(
        &self,
        writer: &mut W,
    ) -> Result<usize, tls_codec::Error> {
        match self {
            SinglePart::TextMarkdown(content) => {
                let mut written = ContentType::TextMarkdown.tls_serialize(writer)?;
                written += content.as_bytes().tls_serialize(writer)?;
                Ok(written)
            }
        }
    }
}

impl DeserializeBytes for SinglePart {
    fn tls_deserialize_bytes(buffer: &[u8]) -> Result<(Self, &[u8]), tls_codec::Error> {
        let (content_type, buffer) = ContentType::tls_deserialize_bytes(buffer)?;
        match content_type {
            ContentType::TextMarkdown => {
                let (content, buffer) = TlsStrOwned::tls_deserialize_bytes(buffer)?;
                Ok((SinglePart::TextMarkdown(content.value), buffer))
            }
        }
    }
}
