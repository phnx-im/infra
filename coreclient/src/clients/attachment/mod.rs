// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub use download::{DownloadProgress, DownloadProgressEvent};
pub(crate) use persistence::AttachmentRecord;
pub use persistence::{AttachmentContent, AttachmentStatus};

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
