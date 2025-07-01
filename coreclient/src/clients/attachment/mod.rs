// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{fmt, str::FromStr};

pub use download::{DownloadProgress, DownloadProgressEvent};
pub(crate) use persistence::AttachmentRecord;
pub use persistence::{AttachmentContent, AttachmentStatus};
use phnxcommon::identifiers::{AttachmentId, AttachmentIdParseError};
use thiserror::Error;
use url::Url;

mod content;
mod download;
mod ear;
mod persistence;
mod process;
mod upload;

#[derive(derive_more::From)]
struct AttachmentBytes {
    bytes: Vec<u8>,
}

impl AttachmentBytes {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl AsRef<[u8]> for AttachmentBytes {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug)]
pub struct AttachmentUrl {
    attachment_id: AttachmentId,
    dimensions: Option<(u32, u32)>,
}

impl AttachmentUrl {
    pub fn new(attachment_id: AttachmentId, dimensions: Option<(u32, u32)>) -> Self {
        Self {
            attachment_id,
            dimensions,
        }
    }

    pub fn from_url(url: &Url) -> Result<Self, AttachmentUrlParseError> {
        let attachment_id = AttachmentId::from_url(url)?;

        let width = url
            .query_pairs()
            .find_map(|(key, value)| (key == "width").then(|| value.parse::<u32>().ok())?);
        let dimensions = width.and_then(|width| {
            let height = url
                .query_pairs()
                .find_map(|(key, value)| (key == "height").then(|| value.parse::<u32>().ok())?)?;
            Some((width, height))
        });

        Ok(Self {
            attachment_id,
            dimensions,
        })
    }

    pub fn attachment_id(&self) -> AttachmentId {
        self.attachment_id
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.dimensions
    }
}

impl FromStr for AttachmentUrl {
    type Err = AttachmentUrlParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s)?;
        Self::from_url(&url)
    }
}

#[derive(Debug, Error)]
pub enum AttachmentUrlParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    AttachmentId(#[from] AttachmentIdParseError),
}

impl fmt::Display for AttachmentUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "phnx:///attachment/{}", self.attachment_id.uuid)?;
        if let Some((width, height)) = self.dimensions {
            write!(f, "?width={width}&height={height}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use uuid::uuid;

    #[test]
    fn attachment_url() {
        let id = uuid!("b6a42a7a-62fa-4c10-acfb-6124d80aae09");
        let url = "phnx:///attachment/b6a42a7a-62fa-4c10-acfb-6124d80aae09"
            .parse()
            .unwrap();
        let attachment_id = AttachmentId::from_url(&url).unwrap();
        assert_eq!(attachment_id.uuid, id);

        let attachment_url = AttachmentUrl::new(attachment_id, None);
        assert_eq!(attachment_url.to_string(), url.to_string());
    }

    #[test]
    fn attachment_url_with_dimensions() {
        let id = uuid!("b6a42a7a-62fa-4c10-acfb-6124d80aae09");
        let url = "phnx:///attachment/b6a42a7a-62fa-4c10-acfb-6124d80aae09?width=1920&height=1080"
            .parse()
            .unwrap();
        let attachment_id = AttachmentId::from_url(&url).unwrap();
        assert_eq!(attachment_id.uuid, id);

        let attachment_url = AttachmentUrl::new(attachment_id, Some((1920, 1080)));
        assert_eq!(attachment_url.to_string(), url.to_string());
    }
}
